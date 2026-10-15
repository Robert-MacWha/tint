use std::array::repeat;

use alloy_primitives::{Address, B256, Bytes, keccak256};
use alloy_provider::network::TransactionBuilder;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_sol_types::{SolCall, SolValue};
use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::Groth16;
use ark_hybrid_compression::circuit::Compressed;
use ark_snark::SNARK;
use ark_std::rand::Rng;
use rand_core::{CryptoRng, RngCore};
use tint_groth16::{artifacts::Artifacts, prove::prove_with_matrices};
use tint_poseidon2::poseidon2_compress;
use tracing::info;

use crate::{
    abis::tint::{IPrivacyPool, Tint},
    account::pub_account::PubAccount,
    circuit::join_split::{
        FRONTIER_OFFSET, JoinSplit, JoinSplitCircuit, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS,
    },
    fr::{fr_to_b256, fr_to_u256},
    note::{
        asset::AssetId,
        commitment::BaseCommitment,
        owned::{OwnedNote, SpendableNote},
        withdrawal::Withdrawal,
    },
    skew_mmr::{SkewMmr, SkewMmrProof},
};

/// Collects the inputs, outputs, and withdrawals of an operation.
#[derive(Default)]
pub struct OperationBuilder {
    inputs: Vec<OwnedNote>,
    outputs: Vec<(PubAccount, AssetId, u128)>,
    withdrawals: Vec<Withdrawal>,
    change: Option<PubAccount>,
}

/// An operation missing its policy authorization inputs.
pub struct UnauthorizedOperation {
    operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

/// An operation's inputs, outputs, and withdrawals, padded to the circuit's
/// slot counts. `ciphertexts[i]` encrypts `output_commitments[i]` to its receiver.
#[derive(Clone, Debug)]
pub struct Operation<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize> {
    pub inputs: [SpendableNote; N_INPUTS],
    pub output_commitments: [BaseCommitment; N_OUTPUTS],
    pub output_withdrawals: [Withdrawal; N_WITHDRAWALS],
    pub ciphertexts: [Bytes; N_OUTPUTS],
}

/// A proven operation, ready to submit.
pub struct ProvenOperation {
    outputs: Vec<BaseCommitment>,
    nullifiers: Vec<Fr>,
    public_signals: Vec<Fr>,
    operation: IPrivacyPool::Operation,
}

#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    #[error("more inputs than the circuit supports")]
    TooManyInputs,
    #[error("more outputs than the circuit supports")]
    TooManyOutputs,
    #[error("more withdrawals than the circuit supports")]
    TooManyWithdrawals,
    #[error("outputs and withdrawals exceed inputs for asset {0:?}")]
    InsufficientFunds(AssetId),
    #[error("inputs exceed outputs and withdrawals for asset {0:?}, and no change receiver is set")]
    Unbalanced(AssetId),
    #[error("expected {expected} policy inputs, got {got}")]
    PolicyInputCount { expected: usize, got: usize },
    #[error("an operation needs at least one input")]
    NoInputs,
    #[error("input {0} is not in the skew-MMR")]
    InputNotFound(usize),
    #[error("expected {expected} inclusion proofs, got {got}")]
    ProofCount { expected: usize, got: usize },
    #[error("inclusion proof {0} does not prove its input's commitment")]
    ProofMismatch(usize),
    #[error("inclusion proof {0} is against a different frontier than proof 0")]
    FrontierMismatch(usize),
    #[error("generated proof is invalid")]
    InvalidProof,
    #[error("commitment error: {0}")]
    Commitment(#[from] crate::note::commitment::CommitmentError),
    #[error("circuit error: {0}")]
    Synthesis(#[from] ark_relations::gr1cs::SynthesisError),
    #[error("hybrid-compression error: {0}")]
    Compression(#[from] ark_hybrid_compression::circuit::CompressError),
}

impl OperationBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spends `note` as an input.
    #[must_use]
    pub fn spend(mut self, note: OwnedNote) -> Self {
        self.inputs.push(note);
        self
    }

    /// Creates a new note for `receiver`.
    #[must_use]
    pub fn output(mut self, receiver: PubAccount, asset: AssetId, amount: u128) -> Self {
        self.outputs.push((receiver, asset, amount));
        self
    }

    /// Unshields `amount` of `asset` to `recipient`.
    #[must_use]
    pub fn withdraw(mut self, recipient: Address, asset: AssetId, amount: u128) -> Self {
        self.withdrawals
            .push(Withdrawal::new(asset, amount, recipient));
        self
    }

    /// Sends any remaining input value, per asset, to `receiver`.
    #[must_use]
    pub fn change_to(mut self, receiver: PubAccount) -> Self {
        self.change = Some(receiver);
        self
    }

    /// Fixes the operation's outputs, drawing their randomness and encrypting
    /// each to its receiver.
    ///
    /// # Errors
    ///
    /// Errors if the operation exceeds the circuit's slot counts or doesn't balance.
    pub fn build<R: RngCore + CryptoRng>(
        self,
        rng: &mut R,
    ) -> Result<UnauthorizedOperation, OperationError> {
        if self.inputs.is_empty() {
            return Err(OperationError::NoInputs);
        }

        let mut outputs = self.change_outputs()?;
        outputs.splice(0..0, self.outputs);

        if self.inputs.len() > N_INPUTS {
            return Err(OperationError::TooManyInputs);
        }
        if outputs.len() > N_OUTPUTS {
            return Err(OperationError::TooManyOutputs);
        }
        if self.withdrawals.len() > N_WITHDRAWALS {
            return Err(OperationError::TooManyWithdrawals);
        }

        let mut commitments = Vec::with_capacity(outputs.len());
        let mut ciphertexts = Vec::with_capacity(outputs.len());
        for (receiver, asset, amount) in outputs {
            let commitment = receiver.commitment(asset, amount, B256::new(rng.r#gen()));
            ciphertexts.push(
                commitment
                    .encrypt(&[receiver.encryption_pub_key], rng)?
                    .into(),
            );
            commitments.push(commitment);
        }

        let inputs = self
            .inputs
            .into_iter()
            .map(|note| note.spendable(Bytes::new()));
        Ok(UnauthorizedOperation {
            operation: Operation {
                inputs: pad(inputs),
                output_commitments: pad(commitments),
                output_withdrawals: pad(self.withdrawals),
                ciphertexts: pad(ciphertexts),
            },
        })
    }

    /// Checks that each asset balances, returning a change output for each
    /// asset with input value left over.
    fn change_outputs(&self) -> Result<Vec<(PubAccount, AssetId, u128)>, OperationError> {
        let mut change = Vec::new();
        for asset in self.assets() {
            let input: u128 = self
                .inputs
                .iter()
                .filter(|n| n.note.asset == asset)
                .map(|n| n.note.amount)
                .sum();
            let output: u128 = self
                .outputs
                .iter()
                .filter(|(_, a, _)| *a == asset)
                .map(|(_, _, amount)| amount)
                .sum();
            let withdrawn: u128 = self
                .withdrawals
                .iter()
                .filter(|w| w.asset == asset)
                .map(|w| w.amount)
                .sum();

            let remainder = input
                .checked_sub(output + withdrawn)
                .ok_or(OperationError::InsufficientFunds(asset))?;
            if remainder == 0 {
                continue;
            }

            let receiver = self.change.ok_or(OperationError::Unbalanced(asset))?;
            change.push((receiver, asset, remainder));
        }

        Ok(change)
    }

    /// Every asset this operation touches, in first-seen order.
    fn assets(&self) -> Vec<AssetId> {
        let inputs = self.inputs.iter().map(|n| n.note.asset);
        let outputs = self.outputs.iter().map(|(_, asset, _)| *asset);
        let withdrawals = self.withdrawals.iter().map(|w| w.asset);

        let mut assets = Vec::new();
        for asset in inputs.chain(outputs).chain(withdrawals) {
            if !assets.contains(&asset) {
                assets.push(asset);
            }
        }
        assets
    }
}

impl UnauthorizedOperation {
    /// The padded operation, as spending policies witness it. Policy inputs
    /// are empty; they don't affect [`Operation::signing_hash`].
    #[must_use]
    pub fn operation(&self) -> &Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
        &self.operation
    }

    /// Attaches one policy input per input note, in input order. Inputs under
    /// [`Policy::NONE`](crate::policy::Policy::NONE) take an empty input.
    ///
    /// # Errors
    ///
    /// Errors if `policy_inputs` doesn't have one entry per input.
    pub fn authorize(
        mut self,
        policy_inputs: Vec<Bytes>,
    ) -> Result<Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>, OperationError> {
        let expected = self.operation.live_inputs().count();
        if policy_inputs.len() != expected {
            return Err(OperationError::PolicyInputCount {
                expected,
                got: policy_inputs.len(),
            });
        }

        let inputs = self.operation.inputs.iter_mut().filter(|i| i.is_live());
        for (input, policy_input) in inputs.zip(policy_inputs) {
            input.policy_input = policy_input;
        }
        Ok(self.operation)
    }
}

impl<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize>
    Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>
{
    /// This operation's binding hash, mirroring [`crate::circuit::operation::OperationVar::hash`].
    pub fn hash(&self) -> Fr {
        let mut hash = Fr::from(0u64);
        for input in &self.inputs {
            let c = if input.note.note.amount == 0 {
                Fr::from(0u64)
            } else {
                input.note.commitment()
            };
            hash = poseidon2_compress(&[hash, c]);
        }
        for output in &self.output_commitments {
            let c = if output.amount == 0 {
                Fr::from(0u64)
            } else {
                output.hash()
            };
            hash = poseidon2_compress(&[hash, c]);
        }
        for withdrawal in &self.output_withdrawals {
            let contribution = if withdrawal.amount == 0 {
                Fr::from(0u64)
            } else {
                withdrawal.hash()
            };
            hash = poseidon2_compress(&[hash, contribution]);
        }
        hash
    }

    /// The message spending policies authorize:
    /// `poseidon2(operationHash, contextHash)`.
    ///
    /// Covers everything in the operation except the policy inputs themselves.
    /// Policy circuits witness the operation and [`Self::context_hash`] to
    /// recompute it, so it is their only operation-specific public input.
    ///
    /// Mirrors `ProofLib.signingHash`.
    #[must_use]
    pub fn signing_hash(&self) -> Fr {
        poseidon2_compress(&[self.hash(), self.context_hash()])
    }

    /// The context fields [`Self::hash`] omits:
    /// `keccak256(abi.encode(ciphertexts)) mod p`.
    #[must_use]
    pub fn context_hash(&self) -> Fr {
        let params = (self.ciphertexts.clone(),);
        Fr::from_be_bytes_mod_order(keccak256(params.abi_encode_params()).as_slice())
    }

    /// The non-padding inputs, in slot order.
    fn live_inputs(&self) -> impl Iterator<Item = &SpendableNote> {
        self.inputs.iter().filter(|i| i.is_live())
    }
}

impl Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
    /// Proves the operation against `mmr`'s current frontier.
    ///
    /// See [`Self::prove`].
    ///
    /// # Errors
    ///
    /// Errors if an input isn't in `mmr`, or proving fails.
    pub fn prove_with_mmr<R: RngCore + CryptoRng>(
        self,
        mmr: &SkewMmr,
        artifacts: &Artifacts<Bn254>,
        rng: &mut R,
    ) -> Result<ProvenOperation, OperationError> {
        let proofs = self
            .live_inputs()
            .enumerate()
            .map(|(i, input)| {
                mmr.prove_element(input.note.commitment())
                    .ok_or(OperationError::InputNotFound(i))
            })
            .collect::<Result<_, _>>()?;

        self.prove(proofs, artifacts, rng)
    }

    /// Proves the operation. `proofs` holds one inclusion proof per live
    /// input, in slot order, all against the same frontier.
    ///
    /// # Errors
    ///
    /// Errors if `proofs` don't match the live inputs or share a frontier, or
    /// proving fails.
    pub fn prove<R: RngCore + CryptoRng>(
        self,
        proofs: Vec<SkewMmrProof>,
        artifacts: &Artifacts<Bn254>,
        rng: &mut R,
    ) -> Result<ProvenOperation, OperationError> {
        check_proofs(self.live_inputs(), &proofs)?;

        let hist_state = proofs[0].state;
        let frontier = proofs[0].roots;
        // The frontier is the tail of `stmt`; only its live roots are sent.
        let live = FRONTIER_OFFSET + hist_state.depth();

        // Padding slots take a default proof, which the circuit skips.
        let mut live_proofs = proofs.into_iter();
        let inclusion_proofs = std::array::from_fn(|i| {
            if !self.inputs[i].is_live() {
                return SkewMmrProof::default();
            }
            live_proofs.next().unwrap_or_default()
        });

        let context = self.context();
        let join_split = JoinSplit::new(
            hist_state,
            frontier,
            bound_params_hash(&context),
            inclusion_proofs,
            self.clone(),
        );

        info!("Proving operation...");
        let mut circuit = JoinSplitCircuit::new((), join_split);
        let Compressed { stmt, beta, .. } = circuit.compress()?;
        let (public_inputs, proof) =
            prove_with_matrices(&artifacts.matrices, &artifacts.pk, &circuit, rng)?;

        // Catch bad witnesses before submitting.
        if !Groth16::<Bn254>::verify(&artifacts.vk, &public_inputs, &proof)? {
            return Err(OperationError::InvalidProof);
        }

        Ok(ProvenOperation {
            nullifiers: self.live_inputs().map(|i| i.note.nullifier()).collect(),
            outputs: self
                .output_commitments
                .into_iter()
                .filter(|o| o.amount != 0)
                .collect(),
            operation: IPrivacyPool::Operation {
                r#pub: stmt[..live].iter().copied().map(fr_to_b256).collect(),
                context,
                proof: proof.into(),
                beta: fr_to_u256(beta),
            },
            public_signals: stmt,
        })
    }

    /// The on-chain context.
    fn context(&self) -> IPrivacyPool::Context {
        IPrivacyPool::Context {
            spendabilityInputs: self.inputs.clone().map(|i| i.policy_input),
            ciphertexts: self.ciphertexts.clone(),
        }
    }
}

impl<const I: usize, const O: usize, const W: usize> Default for Operation<I, O, W> {
    fn default() -> Self {
        Operation {
            inputs: repeat(SpendableNote::default()),
            output_commitments: repeat(BaseCommitment::default()),
            output_withdrawals: repeat(Withdrawal::default()),
            ciphertexts: repeat(Bytes::new()),
        }
    }
}

impl ProvenOperation {
    /// The notes this operation creates.
    #[must_use]
    pub fn outputs(&self) -> &[BaseCommitment] {
        &self.outputs
    }

    /// The nullifiers of the notes this operation spends.
    #[must_use]
    pub fn nullifiers(&self) -> &[Fr] {
        &self.nullifiers
    }

    /// The full public-signal vector the proof attests to. The contract
    /// recomputes this in `computePublicSignals`.
    #[must_use]
    pub fn public_signals(&self) -> &[Fr] {
        &self.public_signals
    }

    #[must_use]
    pub fn operation(&self) -> &IPrivacyPool::Operation {
        &self.operation
    }

    #[must_use]
    pub fn call(&self) -> Tint::operateCall {
        Tint::operateCall::new((self.operation.clone(),))
    }

    /// A transaction submitting this operation to the Tint deployment at `instance`.
    #[must_use]
    pub fn transaction(&self, instance: Address) -> TransactionRequest {
        TransactionRequest::default()
            .with_to(instance)
            .with_input(self.call().abi_encode())
    }
}

/// Checks that `proofs` prove each of `inputs`, in order, against one frontier.
fn check_proofs<'a>(
    inputs: impl Iterator<Item = &'a SpendableNote>,
    proofs: &[SkewMmrProof],
) -> Result<(), OperationError> {
    let inputs: Vec<_> = inputs.collect();
    if inputs.is_empty() {
        return Err(OperationError::NoInputs);
    }
    if proofs.len() != inputs.len() {
        return Err(OperationError::ProofCount {
            expected: inputs.len(),
            got: proofs.len(),
        });
    }

    for (i, (input, proof)) in inputs.iter().zip(proofs).enumerate() {
        if proof.element != input.note.commitment() || !proof.verify() {
            return Err(OperationError::ProofMismatch(i));
        }
        if proof.state != proofs[0].state || proof.roots != proofs[0].roots {
            return Err(OperationError::FrontierMismatch(i));
        }
    }

    Ok(())
}

/// Pads `items` with defaults up to the circuit's slot count `N`. Slot counts
/// are checked in [`OperationBuilder::build`], so no items are dropped.
fn pad<T: Default, const N: usize>(items: impl IntoIterator<Item = T>) -> [T; N] {
    let mut items = items.into_iter();
    std::array::from_fn(|_| items.next().unwrap_or_default())
}

/// Mirrors `ProofLib.toBoundParamsHash`
fn bound_params_hash(context: &IPrivacyPool::Context) -> Fr {
    Fr::from_be_bytes_mod_order(keccak256(context.abi_encode()).as_slice())
}

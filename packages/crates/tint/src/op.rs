//! Builds, authorizes, and proves `Tint.operate` calls.
//!
//! ```text
//! OperationBuilder --build--> UnsignedOperation --authorize--> AuthorizedOperation --prove--> ProvenOperation
//! ```
//!
//! Outputs and their randomness are fixed at `build`, so the operation's hash
//! (and anything a spending policy signs over) is stable before authorization.

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
use tracing::info;

use crate::{
    abis::tint::{IPrivacyPool, Tint},
    account::pub_account::PubAccount,
    circuit::{
        join_split::{
            FRONTIER_OFFSET, JoinSplit, JoinSplitCircuit, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS,
        },
        poseidon2::poseidon2_compress,
    },
    fr::{fr_to_b256, fr_to_u256},
    note::{asset::AssetId, commitment::BaseCommitment, owned::OwnedNote, withdrawal::Withdrawal},
    operation::Operation,
    skew_mmr::{SkewMmr, SkewMmrProof},
};

/// Collects the inputs, outputs, and withdrawals of an operation.
pub struct OperationBuilder {
    instance: Address,
    inputs: Vec<OwnedNote>,
    outputs: Vec<(PubAccount, AssetId, u128)>,
    withdrawals: Vec<(Address, Withdrawal)>,
    change: Option<PubAccount>,
}

/// An operation whose outputs are fixed, awaiting one policy input per input note.
pub struct UnsignedOperation {
    instance: Address,
    inputs: Vec<OwnedNote>,
    outputs: Vec<BaseCommitment>,
    ciphertexts: Vec<Bytes>,
    withdrawals: Vec<(Address, Withdrawal)>,
}

/// An operation with a policy input for every input note, ready to prove.
pub struct AuthorizedOperation {
    unsigned: UnsignedOperation,
    policy_inputs: Vec<Bytes>,
}

/// A proven operation, ready to submit.
pub struct ProvenOperation {
    instance: Address,
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
    pub fn new(instance: Address) -> Self {
        Self {
            instance,
            inputs: Vec::new(),
            outputs: Vec::new(),
            withdrawals: Vec::new(),
            change: None,
        }
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
            .push((recipient, Withdrawal::new(asset, amount)));
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
    ) -> Result<UnsignedOperation, OperationError> {
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

        Ok(UnsignedOperation {
            instance: self.instance,
            inputs: self.inputs,
            outputs: commitments,
            ciphertexts,
            withdrawals: self.withdrawals,
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
                .filter(|(_, w)| w.asset == asset)
                .map(|(_, w)| w.amount)
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
        let withdrawals = self.withdrawals.iter().map(|(_, w)| w.asset);

        let mut assets = Vec::new();
        for asset in inputs.chain(outputs).chain(withdrawals) {
            if !assets.contains(&asset) {
                assets.push(asset);
            }
        }
        assets
    }
}

impl UnsignedOperation {
    #[must_use]
    pub fn inputs(&self) -> &[OwnedNote] {
        &self.inputs
    }

    /// The notes this operation creates.
    #[must_use]
    pub fn outputs(&self) -> &[BaseCommitment] {
        &self.outputs
    }

    /// The padded circuit operation, as spending policies witness it. Policy
    /// inputs are left empty; they don't affect [`Self::hash`].
    #[must_use]
    pub fn operation(&self) -> Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
        self.circuit_operation(&[])
    }

    /// The operation's binding hash, as constrained in-circuit.
    #[must_use]
    pub fn hash(&self) -> Fr {
        self.operation().hash()
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
    /// `keccak256(abi.encode(ciphertexts, unshieldRecipients)) mod p`.
    #[must_use]
    pub fn context_hash(&self) -> Fr {
        let context = self.context(&[]);
        let params = (context.ciphertexts, context.unshieldRecipients);
        Fr::from_be_bytes_mod_order(keccak256(params.abi_encode_params()).as_slice())
    }

    /// Attaches one policy input per input note, in input order. Inputs under
    /// [`Policy::NONE`](crate::policy::Policy::NONE) take an empty input.
    ///
    /// # Errors
    ///
    /// Errors if `policy_inputs` doesn't have one entry per input.
    pub fn authorize(
        self,
        policy_inputs: Vec<Bytes>,
    ) -> Result<AuthorizedOperation, OperationError> {
        if policy_inputs.len() != self.inputs.len() {
            return Err(OperationError::PolicyInputCount {
                expected: self.inputs.len(),
                got: policy_inputs.len(),
            });
        }

        Ok(AuthorizedOperation {
            unsigned: self,
            policy_inputs,
        })
    }

    /// The padded circuit operation. Inputs without a policy input get an
    /// empty one, which doesn't affect [`Operation::hash`].
    fn circuit_operation(
        &self,
        policy_inputs: &[Bytes],
    ) -> Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
        let inputs = self
            .inputs
            .iter()
            .enumerate()
            .map(|(i, note)| note.spendable(policy_inputs.get(i).cloned().unwrap_or_default()));
        let withdrawals = self.withdrawals.iter().map(|(_, w)| w.clone());

        Operation::new(
            pad(inputs),
            pad(self.outputs.iter().copied()),
            pad(withdrawals),
        )
    }

    /// The padded on-chain context.
    fn context(&self, policy_inputs: &[Bytes]) -> IPrivacyPool::Context {
        IPrivacyPool::Context {
            spendabilityInputs: pad(policy_inputs.iter().cloned()),
            ciphertexts: pad(self.ciphertexts.iter().cloned()),
            unshieldRecipients: pad(self.withdrawals.iter().map(|(recipient, _)| *recipient)),
        }
    }
}

impl AuthorizedOperation {
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
            .unsigned
            .inputs
            .iter()
            .enumerate()
            .map(|(i, note)| {
                mmr.prove_element(note.commitment())
                    .ok_or(OperationError::InputNotFound(i))
            })
            .collect::<Result<_, _>>()?;

        self.prove(proofs, artifacts, rng)
    }

    /// Proves the operation. `proofs` holds one inclusion proof per input, in
    /// input order, all against the same frontier.
    ///
    /// # Errors
    ///
    /// Errors if `proofs` don't match the inputs or share a frontier, or proving fails.
    pub fn prove<R: RngCore + CryptoRng>(
        self,
        proofs: Vec<SkewMmrProof>,
        artifacts: &Artifacts<Bn254>,
        rng: &mut R,
    ) -> Result<ProvenOperation, OperationError> {
        let unsigned = self.unsigned;
        check_proofs(&unsigned.inputs, &proofs)?;

        let hist_state = proofs[0].state;
        let frontier = proofs[0].roots;
        // The frontier is the tail of `stmt`; only its live roots are sent.
        let live = FRONTIER_OFFSET + hist_state.depth();

        let context = unsigned.context(&self.policy_inputs);
        let join_split = JoinSplit::new(
            hist_state,
            frontier,
            bound_params_hash(&context),
            pad(proofs),
            unsigned.circuit_operation(&self.policy_inputs),
        );

        info!("Proving operation...");
        let mut circuit = JoinSplitCircuit::new((), join_split);
        let Compressed { stmt, beta, .. } = circuit.compress()?;
        let (public_inputs, proof) =
            prove_with_matrices(&artifacts.matrices, &artifacts.pk, &circuit, rng)?;

        // The release-mode prover doesn't check satisfiability, so catch a bad
        // witness here rather than as an opaque on-chain revert.
        if !Groth16::<Bn254>::verify(&artifacts.vk, &public_inputs, &proof)? {
            return Err(OperationError::InvalidProof);
        }

        Ok(ProvenOperation {
            instance: unsigned.instance,
            nullifiers: unsigned.inputs.iter().map(OwnedNote::nullifier).collect(),
            outputs: unsigned.outputs,
            operation: IPrivacyPool::Operation {
                r#pub: stmt[..live].iter().copied().map(fr_to_b256).collect(),
                context,
                proof: proof.into(),
                beta: fr_to_u256(beta),
            },
            public_signals: stmt,
        })
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
}

impl From<ProvenOperation> for TransactionRequest {
    fn from(op: ProvenOperation) -> Self {
        TransactionRequest::default()
            .with_to(op.instance)
            .with_input(op.call().abi_encode())
    }
}

/// Checks that `proofs` prove each of `inputs`, in order, against one frontier.
fn check_proofs(inputs: &[OwnedNote], proofs: &[SkewMmrProof]) -> Result<(), OperationError> {
    if proofs.len() != inputs.len() {
        return Err(OperationError::ProofCount {
            expected: inputs.len(),
            got: proofs.len(),
        });
    }

    for (i, (input, proof)) in inputs.iter().zip(proofs).enumerate() {
        if proof.element != input.commitment() || !proof.verify() {
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

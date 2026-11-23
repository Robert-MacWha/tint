use std::{array::repeat, iter::repeat_n};

use alloy_primitives::{Address, B256, Bytes, keccak256};
use alloy_provider::network::TransactionBuilder;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_sol_types::{SolCall, SolValue};
use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, Proof};
use ark_hybrid_compression::circuit::Compressed;
use ark_snark::SNARK;
use ark_std::rand::Rng;
use rand_core::{CryptoRng, RngCore};
use tint_groth16::{
    artifacts::Artifacts,
    prove::{placeholder, prove_with_matrices},
};
use tint_poseidon2::poseidon2_compress;
use tracing::info;

use crate::{
    abis::tint::{IPrivacyPool, Tint},
    account::{
        keys::{EncryptionKey, EncryptionPubKey},
        pub_account::PubAccount,
    },
    circuit::join_split::{
        FRONTIER_OFFSET, JoinSplit, JoinSplitCircuit, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS,
    },
    crypto::secret::NoteSecret,
    fr::{fr_to_b256, fr_to_u256},
    note::{
        asset::AssetId,
        commitment::{Commitment, CommitmentError},
        owned::{OwnedNote, SpendableNote},
        withdrawal::Withdrawal,
    },
    policy::Policy,
    skew_mmr::{SkewMmr, SkewMmrProof},
};

/// Collects the inputs, outputs, and withdrawals of an operation.
#[derive(Clone)]
pub struct OperationBuilder {
    /// Encrypts the outputs the sender sends itself, without a KEM.
    sender: EncryptionKey,
    inputs: Vec<OwnedNote>,
    outputs: Vec<PendingOutput>,
    withdrawals: Vec<Withdrawal>,
    change: Option<PubAccount>,
    caller: Address,
    binding: B256,
}

/// A built operation, ready for its policies to authorize and then prove.
pub struct Operation {
    witness: OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    /// Which input slots [`Self::authorize`] has filled.
    authorized: [bool; N_INPUTS],
}

/// An operation's inputs, outputs, and withdrawals, padded to the circuit's
/// slot counts.
#[derive(Clone, Debug)]
pub struct OperationWitness<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
> {
    pub inputs: [SpendableNote; N_INPUTS],
    pub outputs: [Commitment; N_OUTPUTS],
    pub withdrawals: [Withdrawal; N_WITHDRAWALS],
    pub ciphertexts: [Bytes; N_OUTPUTS],
    pub caller: Address,
    pub binding: B256,
}

/// A proven operation, ready to submit.
pub struct ProvenOperation {
    outputs: Vec<Commitment>,
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
    #[error("no input is bound to policy {0:?}")]
    UnusedPolicy(Policy),
    #[error("an input bound to policy {0:?} is unauthorized")]
    Unauthorized(Policy),
    #[error("an operation needs at least one input")]
    NoInputs,
    #[error("outputs go to more than one receiver besides the sender")]
    TooManyReceivers,
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

/// An output [`OperationBuilder::build`] will create.
#[derive(Clone)]
#[expect(clippy::large_enum_variant, reason = "an operation has a few outputs")]
enum PendingOutput {
    /// A new note for a receiver, with fresh randomness.
    New(PubAccount, AssetId, u128),
    /// An exact note its receiver already knows.
    Exact(Commitment),
}

impl OperationBuilder {
    /// Starts an operation sent by the holder of `sender`.
    #[must_use]
    pub fn new(sender: &EncryptionKey) -> Self {
        Self {
            sender: sender.clone(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            withdrawals: Vec::new(),
            change: None,
            caller: Address::ZERO,
            binding: B256::ZERO,
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
        self.outputs
            .push(PendingOutput::New(receiver, asset, amount));
        self
    }

    /// Creates exactly `note`, which its receiver must already know.
    ///
    /// The note isn't encrypted to anyone, so it doesn't count as a receiver:
    /// its output carries a ciphertext of the usual length no one can open.
    #[must_use]
    pub fn output_note(mut self, note: Commitment) -> Self {
        self.outputs.push(PendingOutput::Exact(note));
        self
    }

    /// Unshields `amount` of `asset` to `recipient`.
    #[must_use]
    pub fn withdraw(mut self, recipient: Address, asset: AssetId, amount: u128) -> Self {
        self.withdrawals
            .push(Withdrawal::new(asset, amount, recipient));
        self
    }

    /// Restricts execution to `caller`, which the proof commits to.
    #[must_use]
    pub fn caller(mut self, caller: Address) -> Self {
        self.caller = caller;
        self
    }

    /// Sets the operation's `binding`, which the proof commits to.
    #[must_use]
    pub fn binding(mut self, binding: B256) -> Self {
        self.binding = binding;
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
    pub fn build<R: RngCore + CryptoRng>(self, rng: &mut R) -> Result<Operation, OperationError> {
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

        let notes: Vec<_> = outputs.into_iter().map(|o| o.into_note(rng)).collect();
        let ciphertexts = encrypt_outputs(&self.sender, &notes, rng)?;
        let commitments = notes.into_iter().map(|(_, commitment)| commitment);

        let inputs = self
            .inputs
            .into_iter()
            .map(|note| note.spendable(Bytes::new()));
        Ok(Operation {
            witness: OperationWitness {
                inputs: pad(inputs),
                outputs: pad(commitments),
                withdrawals: pad(self.withdrawals),
                ciphertexts: pad(ciphertexts),
                caller: self.caller,
                binding: self.binding,
            },
            authorized: [false; N_INPUTS],
        })
    }

    /// Checks that each asset balances, returning a change output for each
    /// asset with input value left over.
    fn change_outputs(&self) -> Result<Vec<PendingOutput>, OperationError> {
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
                .filter(|o| o.asset() == asset)
                .map(PendingOutput::amount)
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

            let receiver = self
                .change
                .clone()
                .ok_or(OperationError::Unbalanced(asset))?;
            change.push(PendingOutput::New(receiver, asset, remainder));
        }

        Ok(change)
    }

    /// Every asset this operation touches, in first-seen order.
    fn assets(&self) -> Vec<AssetId> {
        let inputs = self.inputs.iter().map(|n| n.note.asset);
        let outputs = self.outputs.iter().map(PendingOutput::asset);
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

impl Operation {
    /// The padded operation, as spending policies witness it. Policy inputs
    /// don't affect [`OperationWitness::signing_hash`], so authorizing doesn't change it.
    #[must_use]
    pub fn witness(&self) -> &OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
        &self.witness
    }

    /// Authorizes every live input bound to `policy` with `input`.
    ///
    /// # Errors
    ///
    /// Errors if no live input is bound to `policy`.
    pub fn authorize(&mut self, policy: Policy, input: Bytes) -> Result<(), OperationError> {
        let inputs = &self.witness.inputs;
        let slots: Vec<usize> = (0..N_INPUTS)
            .filter(|&slot| inputs[slot].is_live() && inputs[slot].note.policy == policy)
            .collect();
        if slots.is_empty() {
            return Err(OperationError::UnusedPolicy(policy));
        }

        for (&slot, input) in slots.iter().zip(repeat_n(input, slots.len())) {
            self.witness.inputs[slot].policy_input = input;
            self.authorized[slot] = true;
        }
        Ok(())
    }

    /// Proves the operation against `mmr`'s current frontier.
    ///
    /// # Errors
    ///
    /// Errors if an input is unauthorized or isn't in `mmr`, or proving fails.
    pub fn prove_with_mmr<R: RngCore + CryptoRng>(
        self,
        mmr: &SkewMmr,
        artifacts: &Artifacts<Bn254>,
        rng: &mut R,
    ) -> Result<ProvenOperation, OperationError> {
        self.into_authorized()?.prove_with_mmr(mmr, artifacts, rng)
    }

    /// Assembles an operation for simulation.
    ///
    /// The resulting [`ProvenOperation`] has placeholder proofs. It can be used for simulation, but
    /// will revert if submitted normally. Use [`Self::prove_with_mmr`] to generate a valid proof
    /// for submission.
    ///
    /// # Errors
    ///
    /// Errors if an input is unauthorized or isn't in `mmr`, or witnessing fails.
    pub fn simulate_with_mmr(self, mmr: &SkewMmr) -> Result<ProvenOperation, OperationError> {
        self.into_authorized()?.simulate_with_mmr(mmr)
    }

    /// The operation, once every live input under a policy is authorized.
    fn into_authorized(
        self,
    ) -> Result<OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>, OperationError> {
        for (slot, note) in self.witness.inputs.iter().enumerate() {
            let policy = note.note.policy;
            if note.is_live() && policy != Policy::NONE && !self.authorized[slot] {
                return Err(OperationError::Unauthorized(policy));
            }
        }
        Ok(self.witness)
    }
}

impl<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize>
    OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>
{
    /// This operation's binding hash, mirroring [`crate::circuit::gadgets::operation::OperationVar::hash`].
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
        for output in &self.outputs {
            let c = if output.amount == 0 {
                Fr::from(0u64)
            } else {
                output.hash()
            };
            hash = poseidon2_compress(&[hash, c]);
        }
        for withdrawal in &self.withdrawals {
            let c = if withdrawal.amount == 0 {
                Fr::from(0u64)
            } else {
                withdrawal.hash()
            };
            hash = poseidon2_compress(&[hash, c]);
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

    /// The context fields [`Self::hash`] omits, except policy inputs:
    /// `keccak256(abi.encode(caller, binding, ciphertexts)) mod p`.
    #[must_use]
    pub fn context_hash(&self) -> Fr {
        let params = (self.caller, self.binding, self.ciphertexts.clone());
        Fr::from_be_bytes_mod_order(keccak256(params.abi_encode_params()).as_slice())
    }

    /// The non-padding inputs, in slot order.
    pub fn live_inputs(&self) -> impl Iterator<Item = &SpendableNote> {
        self.inputs.iter().filter(|i| i.is_live())
    }
}

impl OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
    /// Proves the operation against `mmr`'s current frontier.
    fn prove_with_mmr<R: RngCore + CryptoRng>(
        self,
        mmr: &SkewMmr,
        artifacts: &Artifacts<Bn254>,
        rng: &mut R,
    ) -> Result<ProvenOperation, OperationError> {
        let proofs = self.inclusion_proofs(mmr)?;
        self.prove(proofs, artifacts, rng)
    }

    /// Assembles the operation against `mmr`'s current frontier, with a placeholder proof.
    fn simulate_with_mmr(self, mmr: &SkewMmr) -> Result<ProvenOperation, OperationError> {
        let proofs = self.inclusion_proofs(mmr)?;
        self.assemble(proofs, |_| Ok(placeholder()))
    }

    /// Proves the operation using the given inclusion proofs.
    fn prove<R: RngCore + CryptoRng>(
        self,
        proofs: Vec<SkewMmrProof>,
        artifacts: &Artifacts<Bn254>,
        rng: &mut R,
    ) -> Result<ProvenOperation, OperationError> {
        self.assemble(proofs, |circuit| {
            info!("Proving operation...");
            let (public_inputs, proof) =
                prove_with_matrices(&artifacts.matrices, &artifacts.pk, circuit, rng)?;

            // Catch bad witnesses before submitting.
            if !Groth16::<Bn254>::verify(&artifacts.vk, &public_inputs, &proof)? {
                return Err(OperationError::InvalidProof);
            }
            Ok(proof)
        })
    }

    /// Inclusion proofs for the live inputs against `mmr`'s current frontier.
    fn inclusion_proofs(&self, mmr: &SkewMmr) -> Result<Vec<SkewMmrProof>, OperationError> {
        self.live_inputs()
            .enumerate()
            .map(|(i, input)| {
                mmr.prove_element(input.note.commitment())
                    .ok_or(OperationError::InputNotFound(i))
            })
            .collect()
    }

    /// Witnesses the operation against `proofs` and attaches the Groth16
    /// proof `prove` returns for the compressed circuit.
    fn assemble(
        self,
        proofs: Vec<SkewMmrProof>,
        prove: impl FnOnce(&JoinSplitCircuit) -> Result<Proof<Bn254>, OperationError>,
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

        let mut circuit = JoinSplitCircuit::new((), join_split);
        let Compressed { stmt, beta, .. } = circuit.compress()?;
        let proof = prove(&circuit)?;

        Ok(ProvenOperation {
            nullifiers: self.live_inputs().map(|i| i.note.nullifier()).collect(),
            outputs: self.outputs.into_iter().filter(|o| o.amount != 0).collect(),
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
            policyInputs: self.inputs.clone().map(|i| i.policy_input),
            ciphertexts: self.ciphertexts.clone(),
            caller: self.caller,
            binding: self.binding,
        }
    }
}

impl<const I: usize, const O: usize, const W: usize> Default for OperationWitness<I, O, W> {
    fn default() -> Self {
        OperationWitness {
            inputs: repeat(SpendableNote::default()),
            outputs: repeat(Commitment::default()),
            withdrawals: repeat(Withdrawal::default()),
            ciphertexts: repeat(Bytes::new()),
            caller: Address::ZERO,
            binding: B256::ZERO,
        }
    }
}

impl ProvenOperation {
    /// The notes this operation creates.
    #[must_use]
    pub fn outputs(&self) -> &[Commitment] {
        &self.outputs
    }

    /// The nullifiers of the notes this operation spends.
    #[must_use]
    pub fn nullifiers(&self) -> &[Fr] {
        &self.nullifiers
    }

    /// The full public-signal vector the proof attests to. The contract
    /// recomputes this in `ProofLib.toPublicSignals`.
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

impl PendingOutput {
    fn asset(&self) -> AssetId {
        match self {
            Self::New(_, asset, _) => *asset,
            Self::Exact(note) => note.asset,
        }
    }

    fn amount(&self) -> u128 {
        match self {
            Self::New(_, _, amount) => *amount,
            Self::Exact(note) => note.amount,
        }
    }

    /// The note to create, and the key to encrypt it to unless it's exact.
    fn into_note<R: RngCore + CryptoRng>(
        self,
        rng: &mut R,
    ) -> (Option<EncryptionPubKey>, Commitment) {
        match self {
            Self::New(receiver, asset, amount) => {
                let note = receiver.commitment(asset, amount, B256::new(rng.r#gen()));
                (Some(receiver.encryption_pub_key), note)
            }
            Self::Exact(note) => (None, note),
        }
    }
}

/// Checks that `proofs` prove each of `inputs` against a shared frontier.
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

/// Encrypts each output note, carrying one shared KEM header on the first.
///
/// Notes to the sender are sealed under its self-secret, and exact notes (with
/// no key) under a throwaway secret. The rest must share a single receiver,
/// whom the header encapsulates to (the sender if there's none), so every
/// operation carries exactly one header whoever it pays.
fn encrypt_outputs<R: RngCore + CryptoRng>(
    sender: &EncryptionKey,
    notes: &[(Option<EncryptionPubKey>, Commitment)],
    rng: &mut R,
) -> Result<Vec<Bytes>, OperationError> {
    if notes.is_empty() {
        return Ok(Vec::new());
    }

    let sender_pub = sender.public_key();
    let mut others = notes
        .iter()
        .filter_map(|(r, _)| r.as_ref())
        .filter(|r| **r != sender_pub);
    let receiver = others.next().unwrap_or(&sender_pub);
    if others.any(|r| r != receiver) {
        return Err(OperationError::TooManyReceivers);
    }

    let (shared, header) = receiver.encapsulate(rng).map_err(CommitmentError::from)?;
    let throwaway = NoteSecret::new(rng.r#gen());
    notes
        .iter()
        .enumerate()
        .map(|(i, (receiver, note))| {
            let secret = match receiver {
                None => &throwaway,
                Some(r) if *r == sender_pub => sender.self_secret(),
                Some(_) => &shared,
            };
            let mut ciphertext = if i == 0 { header.to_vec() } else { Vec::new() };
            ciphertext.extend(note.encrypt(secret)?);
            Ok(ciphertext.into())
        })
        .collect()
}

/// Pads `items` with defaults up to the circuit's slot count `N`. Slot counts
/// are checked in [`OperationBuilder::build`];
fn pad<T: Default, const N: usize>(items: impl IntoIterator<Item = T>) -> [T; N] {
    let mut items = items.into_iter();
    std::array::from_fn(|_| items.next().unwrap_or_default())
}

/// Mirrors `ProofLib.toBoundParamsHash`
fn bound_params_hash(context: &IPrivacyPool::Context) -> Fr {
    Fr::from_be_bytes_mod_order(keccak256(context.abi_encode()).as_slice())
}

#[cfg(test)]
mod tests {
    use ark_std::rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::account::Account;

    /// Should authorize a policy's inputs, after which the operation assembles.
    #[test]
    fn test_authorize() {
        let (mut op, mmr) = operation(policy());

        op.authorize(policy(), Bytes::from_static(b"input"))
            .unwrap();

        let simulated = op.simulate_with_mmr(&mmr).unwrap();
        assert_eq!(
            simulated.operation().context.policyInputs[0],
            Bytes::from_static(b"input")
        );
    }

    /// Should refuse to authorize a policy no input is bound to.
    #[test]
    fn test_authorize_unused_policy() {
        let (mut op, _) = operation(policy());
        let other = Policy::new(policy().address, Fr::from(8));

        let err = op.authorize(other, Bytes::new()).unwrap_err();
        assert!(matches!(err, OperationError::UnusedPolicy(p) if p == other));
    }

    /// Should refuse to assemble while an input under a policy is unauthorized.
    #[test]
    fn test_unauthorized() {
        let (op, mmr) = operation(policy());

        let err = op.simulate_with_mmr(&mmr).err().unwrap();
        assert!(matches!(err, OperationError::Unauthorized(p) if p == policy()));
    }

    /// Should need no authorization for inputs under `Policy::NONE`.
    #[test]
    fn test_no_policy() {
        let (op, mmr) = operation(Policy::NONE);

        op.simulate_with_mmr(&mmr).unwrap();
    }

    /// Should encrypt a transfer's outputs for the receiver and the sender, with
    /// one encapsulation header on the first output only.
    #[test]
    fn test_encrypts_outputs() {
        let (sender, note) = funded(1);
        let receiver = Account::from_seed(&[2; 32], Policy::NONE);

        let op = OperationBuilder::new(&sender.encryption_key)
            .spend(note)
            .output(receiver.pub_account(), AssetId::default(), 30)
            .change_to(sender.pub_account())
            .build(&mut StdRng::seed_from_u64(1))
            .unwrap();

        let witness = op.witness();
        let (header, paid) = witness.ciphertexts[0].split_first_chunk().unwrap();
        let shared = receiver.encryption_key.decapsulate(header).unwrap();
        let paid = Commitment::decrypt(paid, witness.outputs[0].hash(), &shared).unwrap();
        let change = Commitment::decrypt(
            &witness.ciphertexts[1],
            witness.outputs[1].hash(),
            sender.encryption_key.self_secret(),
        )
        .unwrap();
        assert_eq!(paid, witness.outputs[0]);
        assert_eq!(change, witness.outputs[1]);
    }

    /// Should refuse outputs to more than one receiver besides the sender.
    #[test]
    fn test_too_many_receivers() {
        let (sender, note) = funded(1);
        let receiver_1 = Account::from_seed(&[2; 32], Policy::NONE);
        let receiver_2 = Account::from_seed(&[3; 32], Policy::NONE);

        let err = OperationBuilder::new(&sender.encryption_key)
            .spend(note)
            .output(receiver_1.pub_account(), AssetId::default(), 30)
            .output(receiver_2.pub_account(), AssetId::default(), 70)
            .build(&mut StdRng::seed_from_u64(1))
            .err()
            .unwrap();

        assert!(matches!(err, OperationError::TooManyReceivers));
    }

    /// Should create exact notes as given, keeping their randomness.
    #[test]
    fn test_output_note() {
        let (sender, note) = funded(1);
        let exact = sender
            .pub_account()
            .commitment(AssetId::default(), 100, B256::repeat_byte(9));

        let op = OperationBuilder::new(&sender.encryption_key)
            .spend(note)
            .output_note(exact)
            .build(&mut StdRng::seed_from_u64(1))
            .unwrap();

        assert_eq!(op.witness().outputs[0], exact);
    }

    /// Should not count exact notes as receivers, nor change ciphertext lengths.
    #[test]
    fn test_output_note_not_a_receiver() {
        let (sender, note) = funded(1);
        let exact = |seed| {
            Account::from_seed(&[seed; 32], Policy::NONE)
                .pub_account()
                .commitment(AssetId::default(), 30, B256::repeat_byte(seed))
        };
        let receiver = Account::from_seed(&[4; 32], Policy::NONE);
        let lengths = |op: Operation| op.witness().ciphertexts.clone().map(|c| c.len());

        let exact_op = OperationBuilder::new(&sender.encryption_key)
            .spend(note.clone())
            .output_note(exact(2))
            .output_note(exact(3))
            .output(receiver.pub_account(), AssetId::default(), 40)
            .build(&mut StdRng::seed_from_u64(1))
            .unwrap();
        let plain_op = OperationBuilder::new(&sender.encryption_key)
            .spend(note)
            .output(receiver.pub_account(), AssetId::default(), 30)
            .output(receiver.pub_account(), AssetId::default(), 30)
            .output(receiver.pub_account(), AssetId::default(), 40)
            .build(&mut StdRng::seed_from_u64(1))
            .unwrap();

        assert_eq!(lengths(exact_op), lengths(plain_op));
    }

    fn policy() -> Policy {
        Policy::new(Address::repeat_byte(1), Fr::from(7))
    }

    /// An operation spending one note under `policy`, and an MMR holding it.
    fn operation(policy: Policy) -> (Operation, SkewMmr) {
        let account = Account::from_seed(&[11u8; 32], policy);
        let note = account
            .pub_account()
            .commitment(AssetId::default(), 100, B256::ZERO);
        let note = OwnedNote::new(note, &account).unwrap();

        let mut mmr = SkewMmr::new();
        mmr.append(note.commitment());

        let op = OperationBuilder::new(&account.encryption_key)
            .spend(note)
            .change_to(account.pub_account())
            .build(&mut StdRng::seed_from_u64(1))
            .unwrap();
        (op, mmr)
    }

    /// An account holding a note of 100 under no policy.
    fn funded(seed: u8) -> (Account, OwnedNote) {
        let account = Account::from_seed(&[seed; 32], Policy::NONE);
        let note = account
            .pub_account()
            .commitment(AssetId::default(), 100, B256::ZERO);
        let note = OwnedNote::new(note, &account).unwrap();
        (account, note)
    }
}

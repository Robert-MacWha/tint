use std::array::repeat;

use alloy_primitives::{Address, B256, Bytes, keccak256};
use alloy_sol_types::SolCall;
use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_snark::SNARK;
use ark_std::rand::Rng;
use rand_core::{CryptoRng, RngCore};

use crate::{
    abis::tint::{IPrivacyPool, Tint},
    account::{Account, receiver::Receiver},
    array::try_from_fn,
    circuit::join_split::{JoinSplit, K, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, TREE_DEPTH},
    database::DatabaseError,
    indexer::{Indexer, fr_to_b256, merkle_tree::InclusionProof},
    note::{
        asset::AssetId,
        commitment::{BaseCommitment, Commitment, SpendableCommitment},
        payload::NotePayload,
        withdrawal::Withdrawal,
    },
    operation::Operation,
};

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("more inputs, outputs, or withdrawals than this operation supports")]
    TooManySlots,
    #[error("input commitment not present in the tree — not yet synced, or already spent")]
    InputNotFound,
    #[error("generated proof failed local verification")]
    InvalidProof,
    #[error("indexer error: {0}")]
    Indexer(#[from] crate::indexer::IndexerError),
    #[error("merkle tree error: {0}")]
    MerkleTree(#[from] crate::indexer::merkle_tree::MerkleTreeError),
    #[error("circuit error: {0}")]
    Synthesis(#[from] ark_relations::gr1cs::SynthesisError),
    #[error("note payload error: {0}")]
    NotePayload(#[from] crate::note::payload::NotePayloadError),
}

/// Builds shield/transfer/unshield calls against a Tint deployment.
pub struct Provider {
    pub indexer: Indexer,
    proving_key: ProvingKey<Bn254>,
    verifying_key: VerifyingKey<Bn254>,
}

impl Provider {
    pub fn new(
        indexer: Indexer,
        proving_key: ProvingKey<Bn254>,
        verifying_key: VerifyingKey<Bn254>,
    ) -> Self {
        Self {
            indexer,
            proving_key,
            verifying_key,
        }
    }

    /// Adds an account which will be indexed.
    pub async fn add_account(&mut self, account: Account) -> Result<(), DatabaseError> {
        self.indexer.add_account(account).await
    }

    /// Returns the notes spendable by `receiver`.
    pub fn spendable_notes(&self, receiver: Receiver) -> Vec<&SpendableCommitment> {
        self.indexer.spendable_notes(receiver)
    }

    /// Synchronize the indexer with the on-chain state.
    pub async fn sync(&mut self) -> Result<(), ProviderError> {
        self.indexer.sync().await?;
        Ok(())
    }

    /// Builds a `deposit` call for a new note payable to `receiver`.
    pub fn deposit<R: RngCore + CryptoRng>(
        &self,
        receiver: Receiver,
        asset: AssetId,
        amount: u128,
        rng: &mut R,
    ) -> Result<Tint::depositCall, ProviderError> {
        let random = B256::new(rng.r#gen());
        let commitment = receiver.commitment(asset, amount, random);
        let encrypted_note = NotePayload::from_commitment(&commitment)
            .encrypt(&[receiver.encryption_pub_key], rng)?;

        Ok(Tint::depositCall {
            asset: asset.into(),
            amount,
            partialCommitment: fr_to_b256(commitment.partial_hash()),
            encryptedNote: Bytes::from(encrypted_note),
        })
    }

    /// Builds a proven `operate` call spending `inputs` into `outputs`
    /// (new shielded notes) and `withdrawals` (unshields).
    pub fn operate<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: [SpendableCommitment; I],
        outputs: [(Receiver, AssetId, u128); O],
        withdrawals: [(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<Tint::operateCall, ProviderError> {
        let (operation, _public_inputs) = self.operation(inputs, outputs, withdrawals, rng)?;

        Ok(Tint::operateCall::new((operation,)))
    }

    /// Computes the public-input vector and on-chain `Operation` for
    /// this operation without generating a Groth16 proof.
    pub fn public_inputs<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: [SpendableCommitment; I],
        outputs: [(Receiver, AssetId, u128); O],
        withdrawals: [(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<(Tint::computePublicSignalsCall, Vec<Fr>), ProviderError> {
        let (operation, public_inputs) = self.operation(inputs, outputs, withdrawals, rng)?;

        Ok((
            Tint::computePublicSignalsCall::new((operation,)),
            public_inputs,
        ))
    }

    /// Computes the public-input vector and on-chain `Operation` for
    /// this operation.
    fn operation<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: [SpendableCommitment; I],
        outputs: [(Receiver, AssetId, u128); O],
        withdrawals: [(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<(IPrivacyPool::Operation, Vec<Fr>), ProviderError> {
        let circuit = self.build_circuit(&inputs, &outputs, &withdrawals, rng)?;

        let spendability_inputs = spendability_inputs(&inputs);
        let unshield_recipients = unshield_recipients(&withdrawals);
        let encrypted_notes = try_from_fn(|i| {
            let output = &circuit.operation.output_commitments[i];
            let Some((receiver, _, _)) = outputs.get(i) else {
                //? surprise turbofish
                return Ok::<Bytes, ProviderError>(Bytes::new());
            };
            Ok(Bytes::from(
                NotePayload::from_commitment(output)
                    .encrypt(&[receiver.encryption_pub_key], rng)?,
            ))
        })?;

        let old_root = circuit.old_root;
        let start_aggregation_index = circuit.start_aggregation_index;
        let end_aggregation_index = self.indexer.posted_aggregation_index();

        let public_inputs = circuit.synthesize_public_inputs()?;
        let outputs = circuit.synthesize_outputs()?;
        let proof = Groth16::<Bn254>::prove(&self.proving_key, circuit, rng)?;

        // Smoke-verify the proof locally
        if !Groth16::<Bn254>::verify(&self.verifying_key, &public_inputs, &proof)? {
            return Err(ProviderError::InvalidProof);
        }

        Ok((
            IPrivacyPool::Operation {
                oldRoot: fr_to_b256(old_root),
                startAggregationIndex: start_aggregation_index,
                endAggregationIndex: end_aggregation_index,
                newRoot: fr_to_b256(outputs.new_root),
                nullifiers: outputs.nullifiers.map(fr_to_b256),
                commitmentsOut: outputs.output_commitment_hashes.map(fr_to_b256),
                unshieldAmounts: outputs.withdrawal_amounts,
                unshieldAssets: outputs.withdrawal_assets.map(|a| a.0),
                unshieldRecipients: unshield_recipients,
                spendabilityAddresses: outputs.spendability_addresses,
                spendabilityInputs: spendability_inputs,
                encryptedNotes: encrypted_notes,
                proof: proof.into(),
            },
            public_inputs,
        ))
    }

    /// Builds the `JoinSplit` circuit witnessing `inputs` spent into
    /// `outputs` + `withdrawals`.
    fn build_circuit<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: &[SpendableCommitment; I],
        outputs: &[(Receiver, AssetId, u128); O],
        withdrawals: &[(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<JoinSplit, ProviderError> {
        let old_root = self.indexer.root();
        let start_aggregation_index = self.indexer.posted_aggregation_index();
        let start_aggregation_hash = self.indexer.posted_aggregation_hash();

        let subtree_append = self.indexer.commit()?;
        let commitment_inclusion_proofs = self.commitment_inclusion_proofs(inputs)?;
        let operation = self.build_operation(inputs, outputs, withdrawals, rng)?;

        let unshield_recipients = unshield_recipients(withdrawals);
        let spendability_inputs = spendability_inputs(inputs);
        let bound_params_hash = bound_params_hash(&spendability_inputs, &unshield_recipients);

        let circuit = JoinSplit::new(
            // Public inputs
            old_root,
            start_aggregation_index.into(),
            start_aggregation_hash,
            bound_params_hash,
            // Witnessed values
            subtree_append,
            commitment_inclusion_proofs,
            operation,
        );

        Ok(circuit)
    }

    /// Returns the inclusion proofs for each of the given `inputs` in the current tree.
    fn commitment_inclusion_proofs<const I: usize>(
        &self,
        inputs: &[SpendableCommitment; I],
    ) -> Result<[InclusionProof<{ TREE_DEPTH }, { K }>; N_INPUTS], ProviderError> {
        let mut commitment_inclusion_proofs = repeat(InclusionProof::default());
        for (i, input) in inputs.iter().enumerate() {
            let proof = self
                .indexer
                .prove(input.hash())
                .ok_or(ProviderError::InputNotFound)?;
            commitment_inclusion_proofs[i] = proof;
        }

        Ok(commitment_inclusion_proofs)
    }

    /// Builds an `Operation` from the given inputs, outputs, and withdrawals.
    fn build_operation<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &self,
        inputs: &[SpendableCommitment; I],
        outputs: &[(Receiver, AssetId, u128); O],
        withdrawals: &[(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>, ProviderError> {
        const {
            assert!(I <= N_INPUTS, "too many inputs");
            assert!(O <= N_OUTPUTS, "too many outputs");
            assert!(W <= N_WITHDRAWALS, "too many withdrawals");
        }

        let mut input_commitments = repeat(SpendableCommitment::default());
        for (i, input) in inputs.iter().enumerate() {
            input_commitments[i] = input.clone();
        }

        let mut output_commitments = repeat(BaseCommitment::default());
        for (i, (receiver, asset, amount)) in outputs.iter().enumerate() {
            let random = B256::new(rng.r#gen());
            let commitment = receiver.commitment(*asset, amount.clone(), random);
            output_commitments[i] = commitment;
        }

        let mut output_withdrawals = repeat(Withdrawal::default());
        for (i, (_, asset, amount)) in withdrawals.iter().enumerate() {
            output_withdrawals[i] = Withdrawal::new(*asset, amount.clone());
        }

        Ok(Operation::new(
            input_commitments,
            output_commitments,
            output_withdrawals,
        ))
    }
}

fn spendability_inputs<const I: usize>(inputs: &[SpendableCommitment; I]) -> [Bytes; N_INPUTS] {
    let mut spendability_inputs = repeat(Bytes::new());
    for (i, input) in inputs.iter().enumerate() {
        spendability_inputs[i] = input.spendability_input.clone();
    }
    spendability_inputs
}

fn unshield_recipients<const W: usize>(
    withdrawals: &[(Address, AssetId, u128); W],
) -> [Address; N_WITHDRAWALS] {
    let mut unshield_recipients = repeat(Address::ZERO);
    for (i, (addr, _, _)) in withdrawals.iter().enumerate() {
        unshield_recipients[i] = *addr;
    }
    unshield_recipients
}

/// Mirrors `ProofLib.toBoundParamsHash`
fn bound_params_hash(
    spendability_inputs: &[Bytes; N_INPUTS],
    unshield_recipients: &[Address; N_WITHDRAWALS],
) -> Fr {
    let mut packed = Vec::new();
    for i in 0..N_INPUTS {
        packed.extend_from_slice(&spendability_inputs[i]);
    }
    for i in 0..N_WITHDRAWALS {
        packed.extend_from_slice(unshield_recipients[i].as_slice());
    }
    Fr::from_be_bytes_mod_order(keccak256(&packed).as_slice())
}

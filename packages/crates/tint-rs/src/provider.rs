use std::array::repeat;

use alloy_primitives::{Address, B256, Bytes, keccak256};
use alloy_sol_types::SolCall;
use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_snark::SNARK;
use ark_std::rand::Rng;
use rand_core::{CryptoRng, RngCore};

use crate::{
    abis::tint::{IPrivacyPool, ProofLib, Tint},
    circuit::join_split::{
        JoinSplit, JoinSplitResult, K, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, TREE_DEPTH,
    },
    indexer::{Indexer, fr_to_b256, merkle_tree::InclusionProof},
    note::{
        asset::AssetId,
        commitment::{BaseCommitment, Commitment, SpendableCommitment},
        note_payload::NotePayload,
        receiver::Receiver,
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
    #[error("encryption error: {0}")]
    Encryption(#[from] crate::note::encryption::EncryptionError),
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

    pub fn spendable_notes(&self) -> Vec<&SpendableCommitment> {
        self.indexer.spendable_notes()
    }

    pub async fn sync(&mut self) -> Result<(), ProviderError> {
        self.indexer.sync().await?;
        Ok(())
    }

    /// Builds a `deposit` call for a new note payable to `receiver`.
    pub fn deposit<R: RngCore + CryptoRng>(
        &self,
        receiver: &Receiver,
        asset: AssetId,
        amount: u128,
        rng: &mut R,
    ) -> Result<Tint::depositCall, ProviderError> {
        let random = B256::new(rng.r#gen());
        let commitment = receiver.commitment(asset, amount, random);
        let encrypted_note = NotePayload::from_commitment(&commitment).encrypt(
            &self.indexer.encryption_pub_key(),
            &self.indexer.encryption_pub_key(),
            rng,
        )?;

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
        let leaves_aggregation_index = self.indexer.aggregation_index();
        let (circuit, unshield_recipients) =
            self.build_circuit(&inputs, &outputs, &withdrawals, rng)?;

        let mut spendability_addresses = repeat(Address::ZERO);
        let mut spendability_data = repeat(B256::ZERO);
        let mut encrypted_notes = repeat(Bytes::new());
        for (i, (receiver, _, _)) in outputs.iter().enumerate() {
            let output = &circuit.operation.output_commitments[i];
            spendability_addresses[i] = receiver.spendability_address;
            spendability_data[i] = receiver.spendability_data;

            encrypted_notes[i] = Bytes::from(NotePayload::from_commitment(output).encrypt(
                &self.indexer.encryption_pub_key(),
                &receiver.encryption_pub_key,
                rng,
            )?);
        }

        let old_root = circuit.old_root;
        let old_root_length = circuit.old_root_length;
        let start_aggregation_hash = circuit.start_aggregation_hash;

        let public_inputs = circuit.synthesize_public_inputs()?;
        let outputs = circuit.synthesize_outputs()?;
        let proof = Groth16::<Bn254>::prove(&self.proving_key, circuit, rng)?;

        // Smoke-test the proof locally, to avoid submitting an invalid one
        // on-chain (e.g. from an unbalanced operation).
        if !Groth16::<Bn254>::verify(&self.verifying_key, &public_inputs, &proof)? {
            return Err(ProviderError::InvalidProof);
        }

        let operation = self.construct_solidity_operation(
            old_root,
            old_root_length,
            start_aggregation_hash,
            leaves_aggregation_index,
            unshield_recipients,
            spendability_addresses,
            spendability_data,
            encrypted_notes,
            outputs,
            proof.into(),
        );

        Ok(Tint::operateCall::new((operation,)))
    }

    /// Computes the public-input vector and on-chain `Operation` shape for
    /// this operation without generating a Groth16 proof.
    ///
    /// Mutates `self.indexer` the same way `operate()` does.
    pub fn public_inputs<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: [SpendableCommitment; I],
        outputs: [(Receiver, AssetId, u128); O],
        withdrawals: [(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<(IPrivacyPool::Operation, Vec<Fr>), ProviderError> {
        let leaves_aggregation_index = self.indexer.aggregation_index();
        let (circuit, unshield_recipients) =
            self.build_circuit(&inputs, &outputs, &withdrawals, rng)?;

        let old_root = circuit.old_root;
        let old_root_length = circuit.old_root_length;
        let start_aggregation_hash = circuit.start_aggregation_hash;

        let public_inputs = circuit.synthesize_public_inputs()?;
        let outputs = circuit.synthesize_outputs()?;

        let operation = self.construct_solidity_operation(
            old_root,
            old_root_length,
            start_aggregation_hash,
            leaves_aggregation_index,
            unshield_recipients,
            repeat(Address::ZERO),
            repeat(B256::ZERO),
            repeat(Bytes::new()),
            outputs,
            Proof::default().into(),
        );

        Ok((operation, public_inputs))
    }

    /// Builds the `JoinSplit` circuit witnessing `inputs` spent into
    /// `outputs` + `withdrawals`, shared by [`Self::operate`] and
    /// [`Self::public_inputs`]. Drains the indexer's pending commitments in
    /// the process (via `commit()`).
    fn build_circuit<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: &[SpendableCommitment; I],
        outputs: &[(Receiver, AssetId, u128); O],
        withdrawals: &[(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<(JoinSplit, [Address; N_OUTPUTS]), ProviderError> {
        let old_root = self.indexer.root();
        let old_root_length = self.indexer.leaves();
        let start_aggregation_hash = self.indexer.committed_aggregation_hash();

        let subtree_append = self.indexer.commit()?;
        let commitment_inclusion_proofs = self.construct_commitment_inclusion_proofs(inputs)?;
        let operation = self.construct_operation(inputs, outputs, withdrawals, rng)?;

        let mut unshield_recipients = repeat(Address::ZERO);
        for (i, (receiver, _, _)) in withdrawals.iter().enumerate() {
            unshield_recipients[i] = *receiver;
        }
        let bound_params_hash = bound_params_hash(&unshield_recipients);

        let circuit = JoinSplit {
            // Public inputs
            old_root,
            old_root_length,
            start_aggregation_hash,
            bound_params_hash,

            // Witnessed values
            subtree_append,
            commitment_inclusion_proofs,
            operation,
        };

        Ok((circuit, unshield_recipients))
    }

    fn construct_operation<
        const I: usize,
        const O: usize,
        const W: usize,
        R: RngCore + CryptoRng,
    >(
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

    fn construct_commitment_inclusion_proofs<const I: usize>(
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

    fn construct_solidity_operation(
        &self,
        old_root: Fr,
        old_root_length: u64,
        start_aggregation_hash: Fr,
        leaves_aggregation_index: u64,
        unshield_recipients: [Address; N_OUTPUTS],
        spendability_addresses: [Address; N_OUTPUTS],
        spendability_data: [B256; N_OUTPUTS],
        encrypted_notes: [Bytes; N_OUTPUTS],
        outputs: JoinSplitResult,
        proof: ProofLib::Proof,
    ) -> IPrivacyPool::Operation {
        IPrivacyPool::Operation {
            oldRoot: fr_to_b256(old_root),
            oldRootLength: old_root_length,
            startAggregationHash: fr_to_b256(start_aggregation_hash),
            leavesAggregationIndex: leaves_aggregation_index as u128,
            newRoot: fr_to_b256(outputs.new_root),
            nullifiers: outputs.nullifiers.map(fr_to_b256),
            commitmentsOut: outputs.output_commitment_hashes.map(fr_to_b256),
            unshieldAmounts: outputs.withdrawal_amounts,
            unshieldAssets: outputs.withdrawal_assets,
            unshieldRecipients: unshield_recipients,
            spendabilityAddresses: spendability_addresses,
            spendabilityData: spendability_data,
            encryptedNotes: encrypted_notes,
            proof,
        }
    }
}

/// Mirrors `ProofLib.toBoundParamsHash`
fn bound_params_hash(unshield_recipients: &[Address; N_OUTPUTS]) -> Fr {
    let mut packed = Vec::new();
    for i in 0..N_OUTPUTS {
        packed.extend_from_slice(unshield_recipients[i].as_slice());
    }
    Fr::from_be_bytes_mod_order(keccak256(&packed).as_slice())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use ark_std::rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::{
        circuit::setup_circuits,
        database::memory::MemoryDatabase,
        indexer::{
            syncer::{Event, Syncer},
            verifier::Verifier,
        },
        note::keys::Keys,
    };

    struct QueueSyncer {
        events: Arc<Mutex<Vec<Event>>>,
    }

    #[async_trait::async_trait]
    impl Syncer for QueueSyncer {
        async fn latest_block(
            &self,
        ) -> Result<u64, Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(1)
        }

        async fn sync(
            &self,
            _from: u64,
            _to: u64,
        ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(self.events.lock().unwrap().drain(..).collect())
        }
    }

    struct NoopVerifier;

    #[async_trait::async_trait]
    impl Verifier for NoopVerifier {
        async fn verify(
            &self,
            _block: u64,
            _root: Fr,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(())
        }
    }

    fn make_indexer(keys: Keys, events: Arc<Mutex<Vec<Event>>>) -> Indexer {
        Indexer::new(
            Arc::new(QueueSyncer { events }),
            Arc::new(NoopVerifier),
            Arc::new(MemoryDatabase::default()),
            keys.nullifier_key,
            keys.encryption_key,
        )
    }

    fn receiver_for(keys: &Keys) -> Receiver {
        Receiver {
            nullifier_pub_key: keys.nullifier_key.pub_key(),
            encryption_pub_key: keys.encryption_key.public_key(),
            spendability_address: Address::ZERO,
            spendability_data: Default::default(),
        }
    }

    /// Expect that a deposit, once synced, can be proven and spent into a
    /// transfer to a different recipient via a real Groth16 proof.
    #[test]
    #[ignore = "run with `cargo test --release -- --ignored`"]
    fn deposit_then_transfer() {
        let mut rng = StdRng::seed_from_u64(1);
        let (pk, vk) = setup_circuits().unwrap();

        let sender_keys = Keys::from_seed(&[1u8; 32]);
        let recipient_keys = Keys::from_seed(&[2u8; 32]);
        let events = Arc::new(Mutex::new(Vec::new()));

        let sender_indexer = make_indexer(Keys::from_seed(&[1u8; 32]), events.clone());
        let mut provider = Provider::new(sender_indexer, pk, vk);

        let sender_receiver = receiver_for(&sender_keys);
        let asset = AssetId::from(Address::repeat_byte(0xaa));
        let deposit_call = provider
            .deposit(&sender_receiver, asset, 100, &mut rng)
            .unwrap();

        // Simulate the contract emitting `Deposited` for this call.
        events.lock().unwrap().push(Event::Deposit(Tint::Deposited {
            asset: deposit_call.asset,
            amount: deposit_call.amount,
            partialCommitment: deposit_call.partialCommitment,
            encryptedNote: deposit_call.encryptedNote.clone(),
        }));

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(provider.indexer.sync())
            .unwrap();

        let spendable = provider.indexer.spendable_notes();
        assert_eq!(spendable.len(), 1);
        let input_note = spendable[0].clone();
        assert_eq!(input_note.base.amount, 100);

        let recipient_receiver = receiver_for(&recipient_keys);

        let operate_call = provider
            .operate(
                [input_note],
                [(recipient_receiver, asset, 100)],
                [],
                &mut rng,
            )
            .unwrap();

        let call_op = operate_call.operation;
        assert_ne!(call_op.nullifiers[0], alloy_primitives::B256::ZERO);
        assert_ne!(call_op.commitmentsOut[0], alloy_primitives::B256::ZERO);
        assert!(!call_op.proof.pA[0].is_zero());
    }
}

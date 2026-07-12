use std::array::repeat;

use alloy_primitives::{Address, B256, Bytes, keccak256};
use ark_bn254::{Bn254, Fr};
use ark_ff::{PrimeField, UniformRand};
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_snark::SNARK;
use rand_core::{CryptoRng, RngCore};

use crate::{
    abis::tint::{IPrivacyPool, ProofLib, Tint},
    circuit::join_split::{JoinSplit, K, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
    indexer::{Indexer, fr_to_b256, merkle_tree::InclusionProof},
    note::{
        asset::AssetId,
        commitment::{Commitment, SpendableCommitment},
        encryption::{self, NotePayload},
        receiver::Receiver,
        withdrawal::Withdrawal,
    },
    operation::Operation,
};

/// The RNG seed used for the dev-only trusted setup below
pub const DEV_SETUP_SEED: u64 = 1;

/// Dev-only trusted setup: produces a throwaway proving/verifying key pair
/// for the JoinSplitCircuit.
pub fn setup<R: RngCore + CryptoRng>(
    rng: &mut R,
) -> Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>), ark_relations::gr1cs::SynthesisError> {
    let circuit = JoinSplit::default();
    Groth16::<Bn254>::circuit_specific_setup(circuit, rng)
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("more inputs, outputs, or withdrawals than this operation supports")]
    TooManySlots,
    #[error("input commitment not present in the tree — not yet synced, or already spent")]
    InputNotFound,
    #[error("merkle tree error: {0}")]
    MerkleTree(#[from] crate::indexer::merkle_tree::MerkleTreeError),
    #[error("circuit error: {0}")]
    Synthesis(#[from] ark_relations::gr1cs::SynthesisError),
}

/// Builds shield/transfer/unshield calls against a `Tint` deployment,
/// generating Groth16 proofs from local [`Indexer`] state.
pub struct Provider {
    pub indexer: Indexer,
    proving_key: ProvingKey<Bn254>,
}

impl Provider {
    pub fn new(indexer: Indexer, proving_key: ProvingKey<Bn254>) -> Self {
        Self {
            indexer,
            proving_key,
        }
    }

    /// Builds a `deposit` call for a new note payable to `receiver`.
    pub fn deposit<R: RngCore + CryptoRng>(
        &self,
        receiver: &Receiver,
        asset: AssetId,
        amount: u128,
        rng: &mut R,
    ) -> Tint::depositCall {
        let random = Fr::rand(rng);
        let commitment = receiver.commitment(asset, amount, random);
        let payload = NotePayload::from_commitment(&commitment);
        let encrypted_note = encryption::encrypt(&receiver.encryption_pub_key, &payload, rng);

        Tint::depositCall {
            asset: asset.0,
            amount,
            partialCommitment: fr_to_b256(commitment.partial_hash()),
            encryptedNote: Bytes::from(encrypted_note),
        }
    }

    /// Builds a proven `operate` call spending `inputs` into `outputs`
    /// (new shielded notes) and `withdrawals` (unshields).
    ///
    /// Unused slots are padded with zeros.
    pub fn operate<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: &[SpendableCommitment; I],
        outputs: &[Commitment; O],
        withdrawals: &[Withdrawal; W],
        rng: &mut R,
    ) -> Result<IPrivacyPool::Operation, ProviderError> {
        const {
            assert!(I <= N_INPUTS, "too many inputs");
            assert!(O <= N_OUTPUTS, "too many outputs");
            assert!(W <= N_WITHDRAWALS, "too many withdrawals");
        }

        let old_root = self.indexer.root();
        let old_root_length = self.indexer.leaves();
        let start_aggregation_hash = self.indexer.aggregation_hash();
        let leaves_aggregation_index = self.indexer.aggregation_index();

        // Insert whatever's currently staged (from prior deposits/operations)
        // into the tree; this operation's inputs are proven against the
        // resulting (new) root. Ordering is important so previously unstaged
        // commitments can be spent.
        let subtree_append = self.indexer.commit()?;

        let mut input_commitments = repeat(SpendableCommitment::default());
        let mut commitment_inclusion_proofs = repeat(InclusionProof::default());
        for (i, input) in inputs.iter().enumerate() {
            let proof = self
                .indexer
                .prove(input.hash())
                .ok_or(ProviderError::InputNotFound)?;

            input_commitments[i] = input.clone();
            commitment_inclusion_proofs[i] = proof;
        }

        let mut output_commitments = repeat(Commitment::default());
        let mut encrypted_notes = repeat(Bytes::new());
        let mut spendability_addresses = repeat(Address::ZERO);
        let mut spendability_data = repeat(B256::ZERO);
        for (i, output) in outputs.iter().enumerate() {
            output_commitments[i] = output.clone();
            encrypted_notes[i] = output.encrypted_note.clone();
            spendability_addresses[i] = output.spendability_address;
            spendability_data[i] = output.spendability_data;
        }

        let mut output_withdrawals: [Withdrawal; N_WITHDRAWALS] =
            std::array::from_fn(|_| Withdrawal::default());
        let mut unshield_recipients = [Address::ZERO; N_OUTPUTS];
        for (i, withdrawal) in withdrawals.iter().enumerate() {
            unshield_recipients[i] = withdrawal.to;
            output_withdrawals[i] = withdrawal.clone();
        }

        let operation = Operation::new(input_commitments, output_commitments, output_withdrawals);

        let unshield_amounts_u128: [u128; N_OUTPUTS] =
            std::array::from_fn(|i| operation.output_withdrawals[i].amount);
        let unshield_assets_addr: [Address; N_OUTPUTS] =
            std::array::from_fn(|i| operation.output_withdrawals[i].asset.0);
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

        let outputs = circuit.synthesize_outputs()?;
        let proof = Groth16::<Bn254>::prove(&self.proving_key, circuit, rng)?;

        Ok(IPrivacyPool::Operation {
            oldRoot: fr_to_b256(old_root),
            oldRootLength: old_root_length,
            startAggregationHash: fr_to_b256(start_aggregation_hash),
            leavesAggregationIndex: leaves_aggregation_index as u128,
            newRoot: fr_to_b256(outputs.new_root),
            nullifiers: outputs.nullifiers.map(fr_to_b256),
            commitmentsOut: outputs.output_commitment_hashes.map(fr_to_b256),
            unshieldAmounts: unshield_amounts_u128,
            unshieldAssets: unshield_assets_addr,
            unshieldRecipients: unshield_recipients,
            spendabilityAddresses: spendability_addresses,
            spendabilityData: spendability_data,
            encryptedNotes: encrypted_notes,
            proof: ProofLib::Proof::from(proof),
        })
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
        database::memory::MemoryDatabase,
        indexer::syncer::{Event, Syncer},
        indexer::verifier::Verifier,
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
            _to: u64,
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
            keys.encryption_secret_key,
        )
    }

    fn receiver_for(keys: &Keys) -> Receiver {
        Receiver {
            nullifier_pub_key: keys.nullifier_key.pub_key(),
            encryption_pub_key: keys.encryption_secret_key.public_key(),
            spendability_address: Address::ZERO,
            spendability_data: Default::default(),
        }
    }

    /// Expect that a deposit, once synced, can be proven and spent into a
    /// transfer to a different recipient via a real Groth16 proof.
    #[test]
    fn deposit_then_transfer() {
        let mut rng = StdRng::seed_from_u64(7);
        let (pk, _vk) = setup(&mut rng).unwrap();

        let sender_keys = Keys::from_seed(&[1u8; 32]);
        let recipient_keys = Keys::from_seed(&[2u8; 32]);
        let events = Arc::new(Mutex::new(Vec::new()));

        let sender_indexer = make_indexer(Keys::from_seed(&[1u8; 32]), events.clone());
        let mut provider = Provider::new(sender_indexer, pk);

        let sender_receiver = receiver_for(&sender_keys);
        let asset = AssetId::from(Address::repeat_byte(0xaa));
        let deposit_call = provider.deposit(&sender_receiver, asset, 100, &mut rng);

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
        assert_eq!(input_note.amount, 100);

        let recipient_receiver = receiver_for(&recipient_keys);
        let operation = provider
            .operate(
                vec![input_note],
                vec![(recipient_receiver, asset, 100)],
                vec![],
                &mut rng,
            )
            .unwrap();

        // The nullifier of the spent note should be revealed, and a new
        // output commitment produced.
        assert_ne!(operation.nullifiers[0], alloy_primitives::B256::ZERO);
        assert_ne!(operation.commitmentsOut[0], alloy_primitives::B256::ZERO);
        assert!(!operation.proof.pA[0].is_zero());
    }
}

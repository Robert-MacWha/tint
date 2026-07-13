pub mod merkle_tree;
pub mod syncer;
pub mod verifier;

use std::sync::Arc;

use alloy_primitives::B256;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};

use crate::{
    circuit::{
        join_split::{K, SUBTREE_PATH_LENGTH, SUBTREE_SIZE, TREE_DEPTH},
        poseidon::poseidon_hash,
    },
    database::Database,
    indexer::{
        merkle_tree::{InclusionProof, IncrementalMerkleTree, MerkleTreeError, SubtreeAppendProof},
        syncer::{Event, Syncer},
        verifier::Verifier,
    },
    note::{
        asset::AssetId,
        commitment::{BaseCommitment, SpendableCommitment},
        keys::{EncryptionKey, EncryptionPubKey, NullifierKey},
        note_payload::NotePayload,
    },
};

const LAST_SYNCED_BLOCK_KEY: &[u8] = b"last_synced_block";

pub(crate) fn fr_to_b256(fr: Fr) -> B256 {
    B256::from_slice(&fr.into_bigint().to_bytes_be())
}

pub(crate) fn b256_to_fr(bytes: B256) -> Fr {
    Fr::from_be_bytes_mod_order(bytes.as_slice())
}

/// Indexes on-chain `Tint` events into a local Merkle tree and
/// aggregation-hash chain, and scans encrypted note payloads for ones
/// spendable by `nullifier_key`.
pub struct Indexer {
    syncer: Arc<dyn Syncer + Send + Sync>,
    verifier: Arc<dyn Verifier + Send + Sync>,
    database: Arc<dyn Database + Send + Sync>,
    nullifier_key: NullifierKey,
    encryption_key: EncryptionKey,

    tree: IncrementalMerkleTree<TREE_DEPTH, K>,
    aggregation_hash: Fr,
    committed_aggregation_hash: Fr,
    total_staged: u64,
    pending_commitments: Vec<Fr>,
    last_synced_block: u64,
    owned_notes: Vec<SpendableCommitment>,
    spent_nullifiers: Vec<Fr>,
}

impl Indexer {
    pub fn new(
        syncer: Arc<dyn Syncer + Send + Sync>,
        verifier: Arc<dyn Verifier + Send + Sync>,
        database: Arc<dyn Database + Send + Sync>,
        nullifier_key: NullifierKey,
        encryption_key: EncryptionKey,
    ) -> Self {
        Self {
            syncer,
            verifier,
            database,
            nullifier_key,
            encryption_key,
            tree: IncrementalMerkleTree::new(),
            aggregation_hash: Fr::from(0u64),
            committed_aggregation_hash: Fr::from(0u64),
            total_staged: 0,
            pending_commitments: Vec::new(),
            last_synced_block: 0,
            owned_notes: Vec::new(),
            spent_nullifiers: Vec::new(),
        }
    }

    /// Loads `last_synced_block` from the database, if previously persisted.
    pub async fn load(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if let Some(bytes) = self.database.get(LAST_SYNCED_BLOCK_KEY).await? {
            self.last_synced_block = u64::from_be_bytes(bytes.as_slice().try_into()?);
        }
        Ok(())
    }

    pub fn root(&self) -> Fr {
        self.tree.root()
    }

    /// This indexer's own encryption public key -- needed to encrypt notes
    /// this wallet sends (see [`crate::note::commitment::BaseCommitment::encrypt`]).
    pub fn encryption_pub_key(&self) -> EncryptionPubKey {
        self.encryption_key.public_key()
    }

    pub fn aggregation_hash(&self) -> Fr {
        self.aggregation_hash
    }

    /// The aggregation hash as of the last [`Self::commit`] — i.e. the ring
    /// value *before* the batch that commit will next drain was staged. This
    /// is what a `JoinSplit` proof's `start_aggregation_hash` must chain
    /// forward from to match the on-chain ring.
    pub fn committed_aggregation_hash(&self) -> Fr {
        self.committed_aggregation_hash
    }

    /// Position `aggregation_hash()` sits at in the on-chain aggregation
    /// ring.
    pub fn aggregation_index(&self) -> u64 {
        self.total_staged.saturating_sub(1)
    }

    /// Number of leaves currently in the Merkle tree.
    pub fn leaves(&self) -> u64 {
        self.tree.len() as u64
    }

    /// Notes owned by `nullifier_key` that have been observed on-chain but
    /// not yet spent, per the locally-tracked set of spent nullifiers.
    pub fn spendable_notes(&self) -> Vec<&SpendableCommitment> {
        self.owned_notes
            .iter()
            .filter(|note| !self.spent_nullifiers.contains(&note.nullifier()))
            .collect()
    }

    /// Returns an inclusion proof for `commitment`, if it's present in the tree.
    pub fn prove(&self, commitment: Fr) -> Option<InclusionProof<TREE_DEPTH, K>> {
        let path = self.tree.path(commitment)?;
        Some(self.tree.inclusion(path))
    }

    /// Fetches and applies any new events since the last sync, advancing
    /// `aggregation_hash` and staging their commitments for the next
    /// [`Self::commit`].
    pub async fn sync(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let latest = self.syncer.latest_block().await?;
        if latest <= self.last_synced_block {
            return Ok(());
        }

        let events = self.syncer.sync(self.last_synced_block + 1, latest).await?;
        for event in events {
            self.apply_event(event);
        }

        self.last_synced_block = latest;
        self.database
            .set(LAST_SYNCED_BLOCK_KEY, &latest.to_be_bytes())
            .await?;

        self.verifier.verify(latest).await?;
        Ok(())
    }

    /// Drains up to `SUBTREE_SIZE` pending commitments, inserts them into
    /// the tree, and returns the append proof needed to build the next
    /// JoinSplit witness.
    pub fn commit(
        &mut self,
    ) -> Result<SubtreeAppendProof<SUBTREE_PATH_LENGTH, SUBTREE_SIZE, K>, MerkleTreeError> {
        let count = SUBTREE_SIZE.min(self.pending_commitments.len());
        let drained: Vec<Fr> = self.pending_commitments.drain(..count).collect();

        let mut hash = self.committed_aggregation_hash;
        for commitment in &drained {
            hash = poseidon_hash(&[hash, *commitment]);
        }
        self.committed_aggregation_hash = hash;

        self.tree
            .append_subtree::<SUBTREE_PATH_LENGTH, SUBTREE_SIZE>(&drained)
    }

    fn apply_event(&mut self, event: Event) {
        match event {
            Event::Deposit(d) => {
                let asset_fr = Fr::from(AssetId::from(d.asset));
                let amount_fr = Fr::from(d.amount);
                let partial_fr = b256_to_fr(d.partialCommitment);
                let commitment = poseidon_hash(&[asset_fr, amount_fr, partial_fr]);

                self.stage(commitment);
                self.try_own(&d.encryptedNote);
            }
            Event::Committed(c) => {
                self.stage(b256_to_fr(c.commitment));
                self.try_own(&c.encryptedNote);
            }
            Event::Nullified(n) => {
                self.spent_nullifiers.push(b256_to_fr(n.nullifier));
            }
            Event::Withdrawn(_) => {}
        }
    }

    fn stage(&mut self, commitment: Fr) {
        self.aggregation_hash = poseidon_hash(&[self.aggregation_hash, commitment]);
        self.total_staged += 1;
        self.pending_commitments.push(commitment);
    }

    fn try_own(&mut self, encrypted_note: &[u8]) {
        let Ok(payload) = NotePayload::from_encrypted(encrypted_note, &self.encryption_key) else {
            return;
        };
        let base = BaseCommitment::new(
            payload.asset.into(),
            payload.amount,
            payload.spendability_address,
            payload.spendability_data,
            self.nullifier_key.pub_key(),
            payload.random,
        );
        let commitment = SpendableCommitment::new(base, self.nullifier_key.clone());
        self.owned_notes.push(commitment);
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use alloy_primitives::{Address, Bytes};
    use rand_core::OsRng;

    use super::*;
    use crate::{
        abis::tint::Tint,
        database::memory::MemoryDatabase,
        note::{commitment::Commitment, keys::Keys},
    };

    struct FakeSyncer {
        events: Vec<Event>,
    }

    #[async_trait::async_trait]
    impl Syncer for FakeSyncer {
        async fn latest_block(&self) -> Result<u64, Box<dyn Error + Send + Sync + 'static>> {
            Ok(1)
        }

        async fn sync(
            &self,
            _from: u64,
            _to: u64,
        ) -> Result<Vec<Event>, Box<dyn Error + Send + Sync + 'static>> {
            Ok(self.events.clone())
        }
    }

    struct NoopVerifier;

    #[async_trait::async_trait]
    impl Verifier for NoopVerifier {
        async fn verify(&self, _to: u64) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            Ok(())
        }
    }

    /// Expect that syncing a deposit event for our own note stages its
    /// commitment, advances the aggregation hash, and makes the note
    /// spendable; and that committing inserts it into the tree provably.
    #[tokio::test]
    async fn sync_and_commit_owned_deposit() {
        let recipient = Keys::from_seed(&[9u8; 32]);
        let commitment = BaseCommitment::new(
            AssetId::from(Address::repeat_byte(0xab)),
            10,
            Address::ZERO,
            Default::default(),
            recipient.nullifier_key.pub_key(),
            B256::new([1; 32]),
        );

        let payload = NotePayload::from_commitment(&commitment);
        let encrypted_note = payload
            .encrypt(
                &recipient.encryption_key.public_key(),
                &recipient.encryption_key.public_key(),
                &mut OsRng,
            )
            .unwrap();

        let deposit_event = Event::Deposit(Tint::Deposited {
            asset: commitment.asset.into(),
            amount: commitment.amount,
            partialCommitment: fr_to_b256(commitment.partial_hash()),
            encryptedNote: Bytes::from(encrypted_note),
        });

        let syncer = Arc::new(FakeSyncer {
            events: vec![deposit_event],
        });
        let verifier = Arc::new(NoopVerifier);
        let database = Arc::new(MemoryDatabase::default());

        let mut indexer = Indexer::new(
            syncer,
            verifier,
            database,
            recipient.nullifier_key,
            recipient.encryption_key,
        );

        indexer.sync().await.unwrap();

        assert_eq!(
            indexer.aggregation_hash(),
            poseidon_hash(&[Fr::from(0u64), commitment.hash()])
        );
        let spendable = indexer.spendable_notes();
        assert_eq!(spendable.len(), 1);
        assert_eq!(spendable[0].hash(), commitment.hash());

        let proof = indexer.commit().unwrap();
        assert_eq!(proof.new_leaves[0], commitment.hash());

        let inclusion = indexer.prove(commitment.hash()).unwrap();
        assert_eq!(inclusion.leaf, commitment.hash());
        assert_eq!(inclusion.root(), indexer.root());
    }

    /// Expect that `committed_aggregation_hash` only advances via `commit()`,
    /// walking exactly the drained batch once -- not double-hashed against
    /// what `aggregation_hash` already folded in at stage time.
    #[test]
    fn committed_aggregation_hash_does_not_double_count() {
        let syncer = Arc::new(FakeSyncer { events: vec![] });
        let verifier = Arc::new(NoopVerifier);
        let database = Arc::new(MemoryDatabase::default());
        let keys = Keys::from_seed(&[3u8; 32]);

        let mut indexer = Indexer::new(
            syncer,
            verifier,
            database,
            keys.nullifier_key,
            keys.encryption_key,
        );

        let a = Fr::from(1u64);
        let b = Fr::from(2u64);
        let c = Fr::from(3u64);

        indexer.stage(a);
        indexer.stage(b);

        let expected_after_stage = poseidon_hash(&[poseidon_hash(&[Fr::from(0u64), a]), b]);
        assert_eq!(indexer.aggregation_hash(), expected_after_stage);
        assert_eq!(indexer.committed_aggregation_hash(), Fr::from(0u64));

        indexer.commit().unwrap();

        // Committing the whole staged batch should bring the checkpoint up
        // to exactly the same value `stage()` already reached -- not past it.
        assert_eq!(indexer.committed_aggregation_hash(), expected_after_stage);
        assert_eq!(indexer.aggregation_hash(), expected_after_stage);

        indexer.stage(c);
        let expected_after_second_stage = poseidon_hash(&[expected_after_stage, c]);
        assert_eq!(indexer.aggregation_hash(), expected_after_second_stage);
        // Not yet committed -- the checkpoint stays behind.
        assert_eq!(indexer.committed_aggregation_hash(), expected_after_stage);

        indexer.commit().unwrap();
        assert_eq!(
            indexer.committed_aggregation_hash(),
            expected_after_second_stage
        );
    }
}

#[cfg(test)]
mod genesis_root {
    use super::*;

    /// Pins the empty tree's root, which `RootRegistry.sol`'s `GENESIS_ROOT`
    /// constant must match exactly — the contract only registers this one
    /// root at deploy time, so the first ever operation's `oldRoot` must
    /// equal it.
    #[test]
    fn matches_solidity_genesis_root() {
        let tree = IncrementalMerkleTree::<TREE_DEPTH, K>::new();
        assert_eq!(
            fr_to_b256(tree.root()),
            "0x2dbd30e0c2cc00efed70e3ffff71cc81d7ea473f78dff9da61e4c9adf9c1a2ed"
                .parse::<B256>()
                .unwrap()
        );
    }
}

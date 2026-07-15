pub mod indexed_account;
pub mod merkle_tree;
pub mod syncer;
pub mod verifier;

use std::sync::Arc;

use alloy_primitives::B256;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use tracing::info;

use crate::{
    account::{Account, receiver::Receiver},
    circuit::{
        join_split::{K, SUBTREE_PATH_LENGTH, SUBTREE_SIZE, TREE_DEPTH},
        poseidon2::poseidon2_hash,
    },
    database::{Database, DatabaseError},
    indexer::{
        indexed_account::IndexedAccount,
        merkle_tree::{InclusionProof, IncrementalMerkleTree, MerkleTreeError, SubtreeAppendProof},
        syncer::{Event, Syncer},
        verifier::Verifier,
    },
    note::commitment::SpendableCommitment,
};

pub(crate) fn fr_to_b256(fr: Fr) -> B256 {
    B256::from_slice(&fr.into_bigint().to_bytes_be())
}

pub(crate) fn b256_to_fr(bytes: B256) -> Fr {
    Fr::from_be_bytes_mod_order(bytes.as_slice())
}

/// Indexes on-chain `Tint` events.
///
/// Maintains a local Merkle tree of note commitments, aggregation hash for
/// staged commitments, and a set of accounts for spendable notes.
pub struct Indexer {
    syncer: Arc<dyn Syncer + Send + Sync>,
    verifier: Arc<dyn Verifier + Send + Sync>,
    database: Arc<dyn Database + Send + Sync>,

    state: IndexerState,
    accounts: Vec<IndexedAccount>,
}

#[derive(Clone, Default, Debug)]
struct IndexerState {
    tree: IncrementalMerkleTree<TREE_DEPTH, K>,

    total_staged: u64,
    staged_commitments: Vec<Fr>,
    posted_aggregation_hash: Fr,

    last_synced_block: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("merkle tree error: {0}")]
    MerkleTree(#[from] MerkleTreeError),
    #[error("syncer error: {0}")]
    Syncer(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("verifier error: {0}")]
    Verifier(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("insufficient staged commitments")]
    InsufficientStagedCommitments,
}

impl Indexer {
    pub fn new(
        syncer: Arc<dyn Syncer + Send + Sync>,
        verifier: Arc<dyn Verifier + Send + Sync>,
        database: Arc<dyn Database + Send + Sync>,
    ) -> Self {
        Self {
            syncer,
            verifier,
            database,
            state: IndexerState {
                tree: IncrementalMerkleTree::new(),
                posted_aggregation_hash: Fr::from(0),
                total_staged: 0,
                staged_commitments: Vec::new(),
                last_synced_block: 0,
            },
            accounts: Vec::new(),
        }
    }

    pub fn root(&self) -> Fr {
        self.state.tree.root()
    }

    /// Returns the currently posted aggregation hash.
    pub fn posted_aggregation_hash(&self) -> Fr {
        self.state.posted_aggregation_hash
    }

    /// Returns the index of the currently posted aggregation hash.
    pub fn posted_aggregation_index(&self) -> u128 {
        self.state.tree.len() as u128
    }

    /// Returns the number of staged commitments that have not been posted.
    pub fn staged_count(&self) -> u128 {
        self.state.staged_commitments.len() as u128
    }

    /// Returns the notes spendable by `receiver`.
    pub fn spendable_notes(&self, receiver: Receiver) -> Vec<&SpendableCommitment> {
        for account in &self.accounts {
            if account.receiver() == receiver {
                return account.spendable_notes();
            }
        }

        Vec::new()
    }

    /// Adds an account which will be indexed.
    pub fn add_account(&mut self, account: Account) {
        self.accounts.push(IndexedAccount::new(account));
    }

    /// Returns an inclusion proof for `commitment`, if it's present in the tree.
    pub fn prove(&self, commitment: Fr) -> Option<InclusionProof<TREE_DEPTH, K>> {
        let path = self.state.tree.path(commitment)?;
        Some(self.state.tree.inclusion(path))
    }

    /// Fetches and applies any new events since the last sync, advancing
    /// `aggregation_hash` and staging their commitments for the next
    /// [`Self::commit`].
    pub async fn sync(&mut self) -> Result<(), IndexerError> {
        let latest = self
            .syncer
            .latest_block()
            .await
            .map_err(IndexerError::Syncer)?;
        if latest <= self.state.last_synced_block {
            info!("No new blocks to sync");
            return Ok(());
        }

        info!(
            "Syncing from block {} to {}",
            self.state.last_synced_block + 1,
            latest
        );

        let events = self
            .syncer
            .sync(self.state.last_synced_block + 1, latest)
            .await
            .map_err(IndexerError::Syncer)?;

        for event in events {
            self.apply_event(event)?;
        }

        self.state.last_synced_block = latest;
        self.verifier
            .verify(latest, self.root())
            .await
            .map_err(IndexerError::Verifier)
    }

    /// Drains up to `SUBTREE_SIZE` pending commitments, inserts them into
    /// the tree, and returns the append proof needed to build the next
    /// JoinSplit witness.
    pub fn commit(
        &mut self,
    ) -> Result<SubtreeAppendProof<SUBTREE_PATH_LENGTH, SUBTREE_SIZE, K>, MerkleTreeError> {
        let count = SUBTREE_SIZE.min(self.state.staged_commitments.len());
        let drained: Vec<Fr> = self.state.staged_commitments.drain(..count).collect();

        let mut hash = self.state.posted_aggregation_hash;
        for commitment in &drained {
            hash = poseidon2_hash(&[hash, *commitment]);
        }
        self.state.posted_aggregation_hash = hash;

        self.state
            .tree
            .append_subtree::<SUBTREE_PATH_LENGTH, SUBTREE_SIZE>(&drained)
    }

    fn apply_event(&mut self, event: Event) -> Result<(), IndexerError> {
        match &event {
            Event::Deposit(d) => {
                self.stage(b256_to_fr(d.commitment));
            }
            Event::Committed(c) => {
                self.stage(b256_to_fr(c.commitment));
            }
            Event::AdvanceAggregationRing(a) => {
                self.advance(a.idx)?;
            }
            Event::Nullified(_) => {}
            Event::Withdrawn(_) => {}
        }

        for account in &mut self.accounts {
            account.apply_event(&event);
        }

        Ok(())
    }

    /// Stage a commitment for inclusion in the next [`Self::commit`].
    fn stage(&mut self, commitment: Fr) {
        self.state.total_staged += 1;
        self.state.staged_commitments.push(commitment);
    }

    /// Advance the aggregation ring to `idx`, inserting all staged commitments
    /// up to that index into the tree.
    fn advance(&mut self, idx: u128) -> Result<(), IndexerError> {
        let stage_count = idx - self.posted_aggregation_index();

        for _ in 0..stage_count {
            if let Some(commitment) = self.state.staged_commitments.pop() {
                self.state.tree.append(&[commitment]);
            } else {
                return Err(IndexerError::InsufficientStagedCommitments);
            }
        }

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use std::error::Error;

//     use alloy_primitives::{Address, Bytes};
//     use rand_core::OsRng;

//     use super::*;
//     use crate::{
//         abis::tint::Tint,
//         account::keys::Keys,
//         database::memory::MemoryDatabase,
//         note::{
//             commitment::{BaseCommitment, Commitment},
//             payload::NotePayload,
//         },
//     };

//     struct FakeSyncer {
//         events: Vec<Event>,
//     }

//     #[async_trait::async_trait]
//     impl Syncer for FakeSyncer {
//         async fn latest_block(&self) -> Result<u64, Box<dyn Error + Send + Sync + 'static>> {
//             Ok(1)
//         }

//         async fn sync(
//             &self,
//             _from: u64,
//             _to: u64,
//         ) -> Result<Vec<Event>, Box<dyn Error + Send + Sync + 'static>> {
//             Ok(self.events.clone())
//         }
//     }

//     struct NoopVerifier;

//     #[async_trait::async_trait]
//     impl Verifier for NoopVerifier {
//         async fn verify(
//             &self,
//             _block: u64,
//             _root: Fr,
//         ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
//             Ok(())
//         }
//     }

//     /// Expect that syncing a deposit event for our own note stages its
//     /// commitment, advances the aggregation hash, and makes the note
//     /// spendable; and that committing inserts it into the tree provably.
//     #[tokio::test]
//     async fn sync_and_commit_owned_deposit() {
//         let recipient = Keys::from_seed(&[9u8; 32]);
//         let commitment = BaseCommitment::new(
//             AssetId::from(Address::repeat_byte(0xab)),
//             10,
//             Address::ZERO,
//             Default::default(),
//             recipient.nullifier_pub_key(),
//             recipient.encryption_pub_key(),
//             B256::new([1; 32]),
//         );

//         let payload = NotePayload::from_commitment(&commitment);
//         let encrypted_note = payload
//             .encrypt(&[recipient.encryption_pub_key()], &mut OsRng)
//             .unwrap();

//         let deposit_event = Event::Deposit(Tint::Deposited {
//             asset: commitment.asset.into(),
//             amount: commitment.amount,
//             partialCommitment: fr_to_b256(commitment.partial_hash()),
//             encryptedNote: Bytes::from(encrypted_note),
//         });

//         let syncer = Arc::new(FakeSyncer {
//             events: vec![deposit_event],
//         });
//         let verifier = Arc::new(NoopVerifier);
//         let database = Arc::new(MemoryDatabase::default());

//         let mut indexer = Indexer::new(syncer, verifier, database);

//         indexer.sync().await.unwrap();

//         assert_eq!(
//             indexer.aggregation_hash(),
//             poseidon2_hash(&[Fr::from(0u64), commitment.hash()])
//         );
//         let spendable = indexer.spendable_notes();
//         assert_eq!(spendable.len(), 1);
//         assert_eq!(spendable[0].hash(), commitment.hash());

//         let proof = indexer.commit().unwrap();
//         assert_eq!(proof.new_leaves[0], commitment.hash());

//         let inclusion = indexer.prove(commitment.hash()).unwrap();
//         assert_eq!(inclusion.leaf, commitment.hash());
//         assert_eq!(inclusion.root(), indexer.root());
//     }

//     /// Expect that `committed_aggregation_hash` only advances via `commit()`,
//     /// walking exactly the drained batch once -- not double-hashed against
//     /// what `aggregation_hash` already folded in at stage time.
//     #[test]
//     fn committed_aggregation_hash_does_not_double_count() {
//         let syncer = Arc::new(FakeSyncer { events: vec![] });
//         let verifier = Arc::new(NoopVerifier);
//         let database = Arc::new(MemoryDatabase::default());
//         let keys = Keys::from_seed(&[3u8; 32]);

//         let mut indexer = Indexer::new(
//             syncer,
//             verifier,
//             database,
//             keys.nullifier_key,
//             keys.encryption_key,
//         );

//         let a = Fr::from(1u64);
//         let b = Fr::from(2u64);
//         let c = Fr::from(3u64);

//         indexer.stage(a);
//         indexer.stage(b);

//         let expected_after_stage = poseidon2_hash(&[poseidon2_hash(&[Fr::from(0u64), a]), b]);
//         assert_eq!(indexer.aggregation_hash(), expected_after_stage);
//         assert_eq!(indexer.committed_aggregation_hash(), Fr::from(0u64));

//         indexer.commit().unwrap();

//         // Committing the whole staged batch should bring the checkpoint up
//         // to exactly the same value `stage()` already reached -- not past it.
//         assert_eq!(indexer.committed_aggregation_hash(), expected_after_stage);
//         assert_eq!(indexer.aggregation_hash(), expected_after_stage);

//         indexer.stage(c);
//         let expected_after_second_stage = poseidon2_hash(&[expected_after_stage, c]);
//         assert_eq!(indexer.aggregation_hash(), expected_after_second_stage);
//         // Not yet committed -- the checkpoint stays behind.
//         assert_eq!(indexer.committed_aggregation_hash(), expected_after_stage);

//         indexer.commit().unwrap();
//         assert_eq!(
//             indexer.committed_aggregation_hash(),
//             expected_after_second_stage
//         );
//     }
// }

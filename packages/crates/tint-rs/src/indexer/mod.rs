pub mod indexed_account;
pub mod merkle_tree;
pub mod syncer;
pub mod verifier;

use std::sync::Arc;

use ark_bn254::Fr;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    account::{Account, receiver::Receiver},
    circuit::{
        join_split::{K, SUBTREE_PATH_LENGTH, SUBTREE_SIZE, TREE_DEPTH},
        poseidon2::poseidon2_compress,
    },
    database::{Database, DatabaseError, TintDatabase},
    fr::b256_to_fr,
    indexer::{
        indexed_account::IndexedAccount,
        merkle_tree::{InclusionProof, IncrementalMerkleTree, MerkleTreeError, SubtreeAppendProof},
        syncer::{Event, Syncer},
        verifier::Verifier,
    },
    note::commitment::SpendableCommitment,
};

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

#[serde_with::serde_as]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct IndexerState {
    tree: IncrementalMerkleTree<TREE_DEPTH, K>,

    total_staged: u64,
    #[serde_as(as = "Vec<crate::serde::fr::FrAsBytes>")]
    staged_commitments: Vec<Fr>,
    #[serde_as(as = "crate::serde::fr::FrAsBytes")]
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
    pub async fn new(
        syncer: Arc<dyn Syncer + Send + Sync>,
        verifier: Arc<dyn Verifier + Send + Sync>,
        database: Arc<dyn Database + Send + Sync>,
    ) -> Result<Self, IndexerError> {
        let state = database.load_indexer().await?.unwrap_or(IndexerState {
            tree: IncrementalMerkleTree::new(),
            total_staged: 0,
            staged_commitments: Vec::new(),
            posted_aggregation_hash: Fr::from(0u64),
            last_synced_block: 0,
        });

        Ok(Self {
            syncer,
            verifier,
            database,
            state,
            accounts: Vec::new(),
        })
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
    pub async fn add_account(&mut self, account: Account) -> Result<(), DatabaseError> {
        let account = IndexedAccount::new(account, self.database.clone()).await?;
        self.accounts.push(account);
        Ok(())
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
            .map_err(IndexerError::Verifier)?;

        self.save().await?;
        Ok(())
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
            hash = poseidon2_compress(&[hash, *commitment]);
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

    async fn save(&self) -> Result<(), DatabaseError> {
        self.database.set_indexer(&self.state).await?;
        for account in &self.accounts {
            account.save().await?;
        }

        Ok(())
    }
}

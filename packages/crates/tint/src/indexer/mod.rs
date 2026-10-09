pub mod indexed_account;
pub mod syncer;
pub mod verifier;

use std::sync::Arc;

use ark_bn254::Fr;
use tracing::info;

use crate::{
    account::{nullifying::NullifyingAccount, receiver::Receiver, viewing::ViewingAccount},
    circuit::{join_split::MMR_MAX_DEPTH, poseidon2::skew_mmr::Poseidon2Hasher},
    fr::b256_to_fr,
    indexer::{
        indexed_account::IndexedAccount,
        syncer::{Event, Syncer},
        verifier::Verifier,
    },
    note::commitment::NullifiableCommitment,
};

/// Indexes on-chain `Tint` events.
///
/// Maintains a local Merkle tree of note commitments, aggregation hash for
/// staged commitments, and a set of accounts for spendable notes.
pub struct Indexer {
    syncer: Arc<dyn Syncer + Send + Sync>,
    verifier: Arc<dyn Verifier + Send + Sync>,

    state: IndexerState,
    accounts: Vec<IndexedAccount>,
}

#[derive(Clone, Default, Debug)]
pub struct IndexerState {
    tree: skew_mmr::SkewMmr<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>,
    last_synced_block: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    // #[error("merkle tree error: {0}")]
    // MerkleTree(#[from] skew_mmr::SkewMmr<>),
    #[error("syncer error: {0}")]
    Syncer(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("verifier error: {0}")]
    Verifier(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("count too large for subtree append: {0}")]
    SubtreeCountTooLarge(usize),
    #[error("insufficient staged commitments")]
    InsufficientStagedCommitments,
}

impl Indexer {
    pub async fn new(
        syncer: Arc<dyn Syncer + Send + Sync>,
        verifier: Arc<dyn Verifier + Send + Sync>,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            syncer,
            verifier,
            state: IndexerState::default(),
            accounts: Vec::new(),
        })
    }

    #[must_use]
    pub fn roots(&self) -> [Option<Fr>; MMR_MAX_DEPTH] {
        self.state.tree.roots()
    }

    #[must_use]
    pub fn ranks(&self) -> [Option<u32>; MMR_MAX_DEPTH] {
        self.state.tree.ranks()
    }

    #[must_use]
    pub fn state(&self) -> skew_mmr::state::State<MMR_MAX_DEPTH> {
        self.state.tree.state()
    }

    /// Returns the notes owned by `receiver`.
    #[must_use]
    pub fn notes(&self, receiver: Receiver) -> Vec<&NullifiableCommitment> {
        for account in &self.accounts {
            if account.matches(&receiver) {
                return account.notes();
            }
        }

        Vec::new()
    }

    /// Adds an account which will be indexed.
    pub async fn add_account(&mut self, viewing: ViewingAccount, nullifying: NullifyingAccount) {
        let account = IndexedAccount::new(viewing, nullifying).await;
        self.accounts.push(account);
    }

    /// Returns an inclusion proof for `commitment`, if it's present in the tree.
    #[must_use]
    pub fn prove(
        &self,
        commitment: Fr,
    ) -> Option<skew_mmr::proof::Proof<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>> {
        self.state.tree.prove_element(commitment)
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

        for event in &events {
            self.apply_event(event)?;
        }

        self.state.last_synced_block = latest;
        self.verifier
            .verify(self.state(), &self.roots())
            .await
            .map_err(IndexerError::Verifier)?;

        Ok(())
    }

    fn apply_event(&mut self, event: &Event) -> Result<(), IndexerError> {
        match event {
            Event::Deposit(d) => {
                self.state.tree.append(b256_to_fr(d.commitment));
            }
            Event::Committed(c) => {
                self.state.tree.append(b256_to_fr(c.commitment));
            }
            Event::Nullified(_) | Event::Withdrawn(_) => {}
        }

        for account in &mut self.accounts {
            account.apply_event(event);
        }

        Ok(())
    }
}

use std::sync::Arc;

use alloy::primitives::{B256, keccak256};
use lean_imt::{
    hashed_tree::{HashedLeanIMT, LeanIMTHasher},
    lean_imt::LeanIMTError,
};

use crate::circuits::inputs::DEPTH;

pub mod syncer;
pub mod verifier;

pub struct Indexer {
    syncer: Arc<dyn syncer::Syncer>,
    verifier: Arc<dyn verifier::Verifier>,

    tree: HashedLeanIMT<32, PoseidonHasher>,
    aggregation_hash: B256,
    pending_commitments: Vec<B256>,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("leanIMTError: {0}")]
    LeanIMTError(#[from] LeanIMTError),
}

struct PoseidonHasher;

impl LeanIMTHasher<32> for PoseidonHasher {
    fn hash(input: &[u8]) -> [u8; 32] {
        // TODO: implement poseidon3 hashing
        let out = keccak256(input);
        out.0
    }
}

impl Indexer {
    pub fn new(syncer: Arc<dyn syncer::Syncer>, verifier: Arc<dyn verifier::Verifier>) -> Self {
        Self {
            syncer,
            verifier,
            tree: HashedLeanIMT::new(&[], PoseidonHasher).unwrap(),
            aggregation_hash: B256::ZERO,
            pending_commitments: Vec::new(),
        }
    }

    /// Returns the latest currently committed root from tint
    pub fn root(&self) -> B256 {
        self.tree.root().map(|r| B256::from(r)).unwrap_or_default()
    }

    /// Returns the latest aggregation hash from tint
    pub fn aggregation_hash(&self) -> B256 {
        self.aggregation_hash
    }

    /// Returns the number of committed leaves
    pub fn leaves(&self) -> u64 {
        self.tree.leaves().len() as u64
    }

    /// Returns the current frontier for the merkle tree.
    ///
    /// The frontier is the list of right-most leaves, used for insertion proofs.
    pub fn frontier(&self) -> [B256; DEPTH] {
        todo!()
    }

    /// Syncs tint to the latest state on-chain.
    pub fn sync(&mut self) -> Result<(), ()> {
        todo!()
    }

    /// Commit the next `n` pending commitments to the merkle tree and aggregation hash.
    ///
    /// If there are fewer than `n` pending commitments, all of them will be committed.
    ///
    /// Returns the committed leaves.
    pub fn commit<const N: usize>(&mut self) -> Result<[B256; N], IndexerError> {
        let drained = self.pending_commitments.drain(..N).collect::<Vec<_>>();
        let mut commitments = [B256::ZERO; N];
        for (i, c) in drained.into_iter().enumerate() {
            commitments[i] = c;
        }

        self.aggregation_hash = aggregate(self.aggregation_hash, &commitments);
        self.tree
            .insert_many(&commitments.iter().map(|c| c.0).collect::<Vec<_>>())?;

        Ok(commitments)
    }
}

fn aggregate(old_aggregation_hash: B256, pending_commitments: &[B256]) -> B256 {
    let mut new_aggregation_hash = old_aggregation_hash;
    for commitment in pending_commitments {
        let mut data = [0u8; 64];
        data[..32].copy_from_slice(&new_aggregation_hash.0);
        data[32..].copy_from_slice(&commitment.0);
        // TODO: use poseidon3 hashing
        new_aggregation_hash = B256::from(keccak256(&data));
    }
    new_aggregation_hash
}

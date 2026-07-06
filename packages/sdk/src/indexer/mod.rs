use std::sync::Arc;

use alloy::primitives::{B256, keccak256};

use crate::circuits::inputs::DEPTH;
use merkle_tree::IncrementalMerkleTree;

pub mod merkle_tree;
pub mod syncer;
pub mod verifier;

pub struct Indexer {
    syncer: Arc<dyn syncer::Syncer>,
    verifier: Arc<dyn verifier::Verifier>,

    tree: IncrementalMerkleTree<DEPTH>,
    aggregation_hash: B256,
    pending_commitments: Vec<B256>,
}

impl Indexer {
    pub fn new(syncer: Arc<dyn syncer::Syncer>, verifier: Arc<dyn verifier::Verifier>) -> Self {
        Self {
            syncer,
            verifier,
            tree: IncrementalMerkleTree::new(),
            aggregation_hash: B256::ZERO,
            pending_commitments: Vec::new(),
        }
    }

    pub fn root(&self) -> B256 {
        B256::from(self.tree.root())
    }

    pub fn aggregation_hash(&self) -> B256 {
        self.aggregation_hash
    }

    pub fn leaves(&self) -> u64 {
        self.tree.size() as u64
    }

    pub fn frontier(&self) -> [B256; DEPTH] {
        self.tree.frontier().map(B256::from)
    }

    // pub fn inclusion_proof(&self, leaf_index: u64) {

    // }

    pub fn sync(&mut self) -> Result<(), ()> {
        todo!()
    }

    pub fn commit<const N: usize>(&mut self) -> Result<[B256; N], std::convert::Infallible> {
        let drained = self.pending_commitments.drain(..N).collect::<Vec<_>>();
        let mut commitments = [B256::ZERO; N];
        for (i, c) in drained.into_iter().enumerate() {
            commitments[i] = c;
        }

        self.aggregation_hash = aggregate(self.aggregation_hash, &commitments);
        self.tree.insert_many(&commitments.map(|c| c.0));

        Ok(commitments)
    }
}

fn aggregate(old_aggregation_hash: B256, pending_commitments: &[B256]) -> B256 {
    let mut new_aggregation_hash = old_aggregation_hash;
    for commitment in pending_commitments {
        let mut data = [0u8; 64];
        data[..32].copy_from_slice(&new_aggregation_hash.0);
        data[32..].copy_from_slice(&commitment.0);
        // TODO: use poseidon2 hashing
        new_aggregation_hash = B256::from(keccak256(&data));
    }
    new_aggregation_hash
}

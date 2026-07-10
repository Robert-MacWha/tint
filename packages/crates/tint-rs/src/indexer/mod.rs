use std::sync::Arc;

use alloy_primitives::{B256, keccak256};

use merkle_tree::IncrementalMerkleTree;

pub mod merkle_tree;
pub mod syncer;
pub mod verifier;

// pub struct Indexer {
//     syncer: Arc<dyn syncer::Syncer>,
//     verifier: Arc<dyn verifier::Verifier>,

//     tree: IncrementalMerkleTree<DEPTH, CHUNK_DEPTH, CHUNK_SIZE, CHUNK_PATH_LEN>,
//     aggregation_hash: B256,
//     pending_commitments: Vec<B256>,
// }

// impl Indexer {
//     pub fn new(syncer: Arc<dyn syncer::Syncer>, verifier: Arc<dyn verifier::Verifier>) -> Self {
//         Self {
//             syncer,
//             verifier,
//             tree: IncrementalMerkleTree::new(),
//             aggregation_hash: B256::ZERO,
//             pending_commitments: Vec::new(),
//         }
//     }

//     pub fn root(&self) -> B256 {
//         B256::from(self.tree.root())
//     }

//     pub fn aggregation_hash(&self) -> B256 {
//         self.aggregation_hash
//     }

//     pub fn leaves(&self) -> u64 {
//         self.tree.size() as u64
//     }

//     pub fn prove(&self, leaf_index: u64) -> Option<[B256; DEPTH]> {
//         self.tree.prove(leaf_index).map(|s| s.map(B256::from))
//     }

//     pub fn sync(&mut self) -> Result<(), ()> {
//         todo!()
//     }

//     /// Drains up to `CHUNK_SIZE` pending commitments, inserts them into the tree,
//     /// and returns the `BatchChunkInsert` witness needed to build the aggregator proof.
//     pub fn commit(&mut self) -> BatchChunkInsertInputs<CHUNK_SIZE, CHUNK_PATH_LEN> {
//         let count = CHUNK_SIZE.min(self.pending_commitments.len());
//         let drained: Vec<B256> = self.pending_commitments.drain(..count).collect();

//         let mut new_leaves = [B256::ZERO; CHUNK_SIZE];
//         for (i, b) in drained.iter().enumerate() {
//             new_leaves[i] = *b;
//         }

//         // Update aggregation hash over the (padded) new leaves.
//         self.aggregation_hash = aggregate(self.aggregation_hash, &new_leaves);
//         self.tree.insert_chunk(&new_leaves)
//     }
// }

// fn aggregate(old_aggregation_hash: B256, pending_commitments: &[B256]) -> B256 {
//     let mut new_aggregation_hash = old_aggregation_hash;
//     for commitment in pending_commitments {
//         let mut data = [0u8; 64];
//         data[..32].copy_from_slice(&new_aggregation_hash.0);
//         data[32..].copy_from_slice(&commitment.0);
//         // TODO: use poseidon2 hashing
//         new_aggregation_hash = B256::from(keccak256(&data));
//     }
//     new_aggregation_hash
// }

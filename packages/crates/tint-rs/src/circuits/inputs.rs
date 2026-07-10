use std::collections::HashMap;

use alloy_primitives::{Address, B256, U256};

use crate::note::asset::AssetId;

pub const N_INPUTS: usize = 5;
pub const N_OUTPUTS: usize = 5;
pub const N_WITHDRAWALS: usize = 5;
pub const DEPTH: usize = 24;
pub const CHUNK_DEPTH: usize = 6;
pub const CHUNK_SIZE: usize = 1 << CHUNK_DEPTH;
pub const CHUNK_PATH_LEN: usize = DEPTH - CHUNK_DEPTH;

/// Circuit input for an `AggregatorProof` invocation.
pub struct AggregatorProofInputs {
    // Public Inputs
    pub old_root: B256,
    pub new_root: B256,
    pub start_aggregation_hash: B256,
    pub end_aggregation_hash: B256,
    pub nullifiers: [B256; N_INPUTS],
    pub commitments_out: [B256; N_OUTPUTS],
    pub unshield_amounts: [u128; N_OUTPUTS],
    pub unshield_assets: [AssetId; N_OUTPUTS],
    pub spendability_hashes_in: [B256; N_INPUTS],
    pub bound_params_hash: B256,

    // Private Inputs
    pub chunk_insert_witness: BatchChunkInsertInputs<CHUNK_SIZE, CHUNK_PATH_LEN>,
    pub siblings_in: [[B256; DEPTH]; N_INPUTS],
    pub leaf_indices_in: [u64; N_INPUTS],
    pub assets_in: [AssetId; N_INPUTS],
    pub amounts_in: [u128; N_INPUTS],
    pub nullifying_keys_in: [B256; N_INPUTS],
    pub random_in: [B256; N_INPUTS],
    pub assets_out: [AssetId; N_OUTPUTS],
    pub amounts_out: [u128; N_OUTPUTS],
    pub nullifying_pub_keys_out: [B256; N_OUTPUTS],
    pub random_out: [B256; N_OUTPUTS],
    pub spendability_hashes_out: [B256; N_OUTPUTS],
}

/// Circuit witness for a `BatchChunkInsert` invocation.
///
/// Provides inputs to prove that a batch of new leaves was inserted into the Merkle tree correctly.
pub struct BatchChunkInsertInputs<const CHUNK_SIZE: usize, const CHUNK_PATH_LEN: usize> {
    pub chunk_index: u64,
    pub chunk_filled: u64,
    pub existing_leaves: [B256; CHUNK_SIZE],
    pub new_leaves: [B256; CHUNK_SIZE],
    pub current_siblings: [B256; CHUNK_PATH_LEN],
    pub next_siblings: [B256; CHUNK_PATH_LEN],
}

// impl AggregatorProofInputs {
//     pub fn inputs_list(self) -> HashMap<String, Vec<U256>> {
//         HashMap::from([
//             ("oldRoot".to_string(), b2u(self.old_root)),
//             ("newRoot".to_string(), b2u(self.new_root)),
//             (
//                 "startAggregationHash".to_string(),
//                 b2u(self.start_aggregation_hash),
//             ),
//             (
//                 "endAggregationHash".to_string(),
//                 b2u(self.end_aggregation_hash),
//             ),
//             ("nullifiers".to_string(), self.nullifiers.map(b2u).concat()),
//             (
//                 "commitmentsOut".to_string(),
//                 self.commitments_out.map(b2u).concat(),
//             ),
//             (
//                 "unshieldAmounts".to_string(),
//                 self.unshield_amounts.map(|x| U256::from(x)).to_vec(),
//             ),
//             (
//                 "unshieldAssets".to_string(),
//                 self.unshield_assets.map(a2u).concat(),
//             ),
//             ("boundParamsHash".to_string(), b2u(self.bound_params_hash)),
//             (
//                 "spendabilityHashesIn".to_string(),
//                 self.spendability_hashes_in.map(b2u).concat(),
//             ),
//             (
//                 "spendabilityHashesOut".to_string(),
//                 self.spendability_hashes_out.map(b2u).concat(),
//             ),
//             (
//                 "newLeaves".to_string(),
//                 self.chunk_insert_witness.new_leaves.map(b2u).concat(),
//             ),
//             (
//                 "currentChunkFilled".to_string(),
//                 vec![U256::from(self.chunk_insert_witness.chunk_filled)],
//             ),
//             (
//                 "currentChunkIndex".to_string(),
//                 vec![U256::from(self.chunk_insert_witness.chunk_index)],
//             ),
//             (
//                 "existingChunkLeaves".to_string(),
//                 self.chunk_insert_witness.existing_leaves.map(b2u).concat(),
//             ),
//             (
//                 "currentChunkSiblings".to_string(),
//                 self.chunk_insert_witness.current_siblings.map(b2u).concat(),
//             ),
//             (
//                 "nextChunkSiblings".to_string(),
//                 self.chunk_insert_witness.next_siblings.map(b2u).concat(),
//             ),
//             (
//                 "siblingsIn".to_string(),
//                 self.siblings_in.map(|x| x.map(b2u).concat()).concat(),
//             ),
//             (
//                 "leafIndicesIn".to_string(),
//                 self.leaf_indices_in.map(|x| U256::from(x)).to_vec(),
//             ),
//             ("assetsIn".to_string(), self.assets_in.map(a2u).concat()),
//             (
//                 "amountsIn".to_string(),
//                 self.amounts_in.map(|x| U256::from(x)).to_vec(),
//             ),
//             (
//                 "nullifyingKeysIn".to_string(),
//                 self.nullifying_keys_in.map(b2u).concat(),
//             ),
//             ("randomIn".to_string(), self.random_in.map(b2u).concat()),
//             ("assetsOut".to_string(), self.assets_out.map(a2u).concat()),
//             (
//                 "amountsOut".to_string(),
//                 self.amounts_out.map(|x| U256::from(x)).to_vec(),
//             ),
//             (
//                 "nullifyingPubKeysOut".to_string(),
//                 self.nullifying_pub_keys_out.map(b2u).concat(),
//             ),
//             ("randomOut".to_string(), self.random_out.map(b2u).concat()),
//         ])
//     }
// }

// /// Converts a B256 into a Vec<U256> for input list
// fn b2u(b: B256) -> Vec<U256> {
//     vec![b.into()]
// }

// fn a2u(a: Address) -> Vec<U256> {
//     vec![U256::from_be_slice(a.as_slice())]
// }

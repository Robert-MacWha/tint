use std::collections::HashMap;

use alloy::primitives::{Address, B256, U256};

pub const N_INPUTS: usize = 5;
pub const N_OUTPUTS: usize = 5;
pub const BATCH_SIZE: usize = 16;
pub const DEPTH: usize = 24;

pub struct AggregatorProofInputs {
    // Public Inputs
    pub old_root: B256,
    pub new_root: B256,
    pub start_aggregation_hash: B256,
    pub end_aggregation_hash: B256,
    pub nullifiers: [B256; N_INPUTS],
    pub commitments_out: [B256; N_OUTPUTS],
    pub unshield_amounts: [u128; N_OUTPUTS],
    pub unshield_assets: [Address; N_OUTPUTS],
    pub bound_params_hash: B256,

    // Private Inputs
    pub batch_start_index: u64,
    pub new_leaves: [B256; BATCH_SIZE],
    pub initial_frontier: [B256; DEPTH],
    pub siblings_in: [[B256; DEPTH]; N_INPUTS],
    pub leaf_indices_in: [u64; N_INPUTS],
    pub assets_in: [Address; N_INPUTS],
    pub amounts_in: [u128; N_INPUTS],
    pub nullifying_keys_in: [B256; N_INPUTS],
    pub partial_commitments_in: [B256; N_INPUTS],
    pub assets_out: [Address; N_OUTPUTS],
    pub amounts_out: [u128; N_OUTPUTS],
    pub partial_commitments_out: [B256; N_OUTPUTS],
}

impl AggregatorProofInputs {
    pub fn inputs_list(self) -> HashMap<String, Vec<U256>> {
        // TODO: Finish this
        HashMap::from([
            ("oldRoot".to_string(), b2u(self.old_root)),
            ("newRoot".to_string(), b2u(self.new_root)),
            (
                "startAggregationHash".to_string(),
                b2u(self.start_aggregation_hash),
            ),
            (
                "endAggregationHash".to_string(),
                b2u(self.end_aggregation_hash),
            ),
            ("nullifiers".to_string(), self.nullifiers.map(b2u).concat()),
            (
                "commitmentsOut".to_string(),
                self.commitments_out.map(b2u).concat(),
            ),
            (
                "unshieldAmounts".to_string(),
                self.unshield_amounts.map(|x| U256::from(x)).to_vec(),
            ),
            (
                "unshieldAssets".to_string(),
                self.unshield_assets.map(a2u).concat(),
            ),
            ("boundParamsHash".to_string(), b2u(self.bound_params_hash)),
            (
                "batchStartIndex".to_string(),
                vec![U256::from(self.batch_start_index)],
            ),
            ("newLeaves".to_string(), self.new_leaves.map(b2u).concat()),
            (
                "initialFrontier".to_string(),
                self.initial_frontier.map(b2u).concat(),
            ),
            (
                "siblingsIn".to_string(),
                self.siblings_in.map(|x| x.map(b2u).concat()).concat(),
            ),
            (
                "leafIndicesIn".to_string(),
                self.leaf_indices_in.map(|x| U256::from(x)).to_vec(),
            ),
            ("assetsIn".to_string(), self.assets_in.map(a2u).concat()),
            (
                "amountsIn".to_string(),
                self.amounts_in.map(|x| U256::from(x)).to_vec(),
            ),
            (
                "nullifyingKeysIn".to_string(),
                self.nullifying_keys_in.map(b2u).concat(),
            ),
            (
                "partialCommitmentsIn".to_string(),
                self.partial_commitments_in.map(b2u).concat(),
            ),
            ("assetsOut".to_string(), self.assets_out.map(a2u).concat()),
            (
                "amountsOut".to_string(),
                self.amounts_out.map(|x| U256::from(x)).to_vec(),
            ),
            (
                "partialCommitmentsOut".to_string(),
                self.partial_commitments_out.map(b2u).concat(),
            ),
        ])
    }
}

/// Converts a B256 into a Vec<U256> for input list
fn b2u(b: B256) -> Vec<U256> {
    vec![b.into()]
}

fn a2u(a: Address) -> Vec<U256> {
    vec![U256::from_be_slice(a.as_slice())]
}

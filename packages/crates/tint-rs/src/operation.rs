use std::array::repeat;

use crate::note::{
    commitment::{BaseCommitment, Commitment, SpendableCommitment},
    withdrawal::Withdrawal,
};

#[derive(Clone, Debug)]
pub struct Operation<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize> {
    pub inputs: [Commitment; N_INPUTS],
    pub output_commitments: [SpendableCommitment; N_OUTPUTS],
    pub output_withdrawals: [Withdrawal; N_WITHDRAWALS],
}

impl<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize>
    Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>
{
    pub fn new(
        inputs: [Commitment; N_INPUTS],
        output_commitments: [SpendableCommitment; N_OUTPUTS],
        output_withdrawals: [Withdrawal; N_WITHDRAWALS],
    ) -> Self {
        Operation {
            inputs,
            output_commitments,
            output_withdrawals,
        }
    }

    // /// Nullifiers for all input notes in this operation.
    // pub fn nullifiers(&self) -> [B256; N_INPUTS] {
    //     let mut nullifiers = [B256::default(); N_INPUTS];
    //     for (i, input) in self.inputs.iter().enumerate() {
    //         nullifiers[i] = input.nullifier();
    //     }
    //     nullifiers
    // }

    // /// Nullifying keys for all input notes in this operation.
    // pub fn nullifying_keys(&self) -> [u128; N_INPUTS] {
    //     let mut nullifying_keys = [0u128; N_INPUTS];
    //     for (i, input) in self.inputs.iter().enumerate() {
    //         nullifying_keys[i] = input.nullifying_key();
    //     }
    //     nullifying_keys
    // }

    // pub fn leaf_indices_in(&self) -> [u64; N_INPUTS] {
    //     let mut leaf_indices = [0u64; N_INPUTS];
    //     for (i, input) in self.inputs.iter().enumerate() {
    //         leaf_indices[i] = input.leaf_index;
    //     }
    //     leaf_indices
    // }

    // /// Assets for all input notes in this operation.
    // pub fn assets_in(&self) -> [AssetId; N_INPUTS] {
    //     let mut assets = [AssetId::default(); N_INPUTS];
    //     for (i, input) in self.inputs.iter().enumerate() {
    //         assets[i] = input.asset;
    //     }
    //     assets
    // }

    // /// Amounts for all input notes in this operation.
    // pub fn amounts_in(&self) -> [u128; N_INPUTS] {
    //     let mut amounts = [0u128; N_INPUTS];
    //     for (i, input) in self.inputs.iter().enumerate() {
    //         amounts[i] = input.amount;
    //     }
    //     amounts
    // }

    // /// Spendability hashes for all input notes in this operation.
    // pub fn spendability_hashes_in(&self) -> [u128; N_INPUTS] {
    //     self.inputs.each_ref().map(|c| c.spendability_hash())
    // }

    // /// Per-note randomness for all input notes in this operation.
    // pub fn random_in(&self) -> [u128; N_INPUTS] {
    //     self.inputs.each_ref().map(|c| c.random)
    // }

    // /// Commitment hashes for all output commitments in this operation.
    // pub fn commitment_hashes(&self) -> [B256; N_OUTPUTS] {
    //     let mut hashes = [B256::default(); N_OUTPUTS];
    //     for (i, output) in self.outputs.iter().enumerate() {
    //         hashes[i] = output.commitment_hash().unwrap_or_default();
    //     }
    //     hashes
    // }

    // /// Assets for all output commitments in this operation.
    // pub fn commitment_assets(&self) -> [AssetId; N_OUTPUTS] {
    //     let mut assets = [AssetId::default(); N_OUTPUTS];
    //     for (i, output) in self.outputs.iter().enumerate() {
    //         assets[i] = output.commitment_asset().unwrap_or_default();
    //     }
    //     assets
    // }

    // /// Amounts for all output commitments in this operation.
    // pub fn commitment_amounts(&self) -> [u128; N_OUTPUTS] {
    //     let mut amounts = [0u128; N_OUTPUTS];
    //     for (i, output) in self.outputs.iter().enumerate() {
    //         amounts[i] = output.commitment_amount().unwrap_or_default();
    //     }
    //     amounts
    // }

    // /// Assets for all output unshields in this operation.
    // pub fn unshield_assets(&self) -> [AssetId; N_OUTPUTS] {
    //     let mut assets = [AssetId::default(); N_OUTPUTS];
    //     for (i, output) in self.outputs.iter().enumerate() {
    //         assets[i] = output.unshield_asset().unwrap_or_default();
    //     }
    //     assets
    // }

    // /// Amounts for all output unshields in this operation
    // pub fn unshield_amounts(&self) -> [u128; N_OUTPUTS] {
    //     let mut amounts = [0u128; N_OUTPUTS];
    //     for (i, output) in self.outputs.iter().enumerate() {
    //         amounts[i] = output.unshield_amount().unwrap_or_default();
    //     }
    //     amounts
    // }

    // /// Spendability hashes for all output commitments in this operation.
    // pub fn spendability_hashes_out(&self) -> [u128; N_OUTPUTS] {
    //     self.outputs
    //         .each_ref()
    //         .map(|o: &Output| o.spendability_hash().unwrap_or_default())
    // }

    // /// Nullifying public keys for all output commitments in this operation.
    // pub fn nullifying_pub_keys_out(&self) -> [B256; N_OUTPUTS] {
    //     self.outputs
    //         .each_ref()
    //         .map(|o| o.nullifying_pub_key().unwrap_or_default())
    // }

    // /// Per-note randomness for all output commitments in this operation.
    // pub fn random_out(&self) -> [u128; N_OUTPUTS] {
    //     self.outputs
    //         .each_ref()
    //         .map(|o| o.random().unwrap_or_default())
    // }
}

impl<const I: usize, const O: usize, const W: usize> Default for Operation<I, O, W> {
    fn default() -> Self {
        Operation {
            inputs: repeat(Commitment::default()),
            output_commitments: repeat(SpendableCommitment::default()),
            output_withdrawals: repeat(Withdrawal::default()),
        }
    }
}

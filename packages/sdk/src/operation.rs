use alloy::primitives::{Address, B256};

use crate::{
    circuits::inputs::{N_INPUTS, N_OUTPUTS},
    note::{commitment::Commitment, withdraw::Withdraw},
};

pub enum Output {
    Commitment(Commitment),
    Withdraw(Withdraw),
}

pub struct Operation {
    pub inputs: [Commitment; N_INPUTS],
    pub outputs: [Output; N_OUTPUTS],
}

impl Operation {
    pub fn new(inputs: [Commitment; N_INPUTS], outputs: [Output; N_OUTPUTS]) -> Self {
        Operation { inputs, outputs }
    }

    /// Nullifiers for all input notes in this operation.
    pub fn nullifiers(&self) -> [B256; N_INPUTS] {
        let mut nullifiers = [B256::default(); N_INPUTS];
        for (i, input) in self.inputs.iter().enumerate() {
            nullifiers[i] = input.nullifier();
        }
        nullifiers
    }

    /// Nullifying keys for all input notes in this operation.
    pub fn nullifying_keys(&self) -> [B256; N_INPUTS] {
        let mut nullifying_keys = [B256::default(); N_INPUTS];
        for (i, input) in self.inputs.iter().enumerate() {
            nullifying_keys[i] = input.nullifying_key();
        }
        nullifying_keys
    }

    /// Assets for all input notes in this operation.
    pub fn assets_in(&self) -> [Address; N_INPUTS] {
        let mut assets = [Address::default(); N_INPUTS];
        for (i, input) in self.inputs.iter().enumerate() {
            assets[i] = input.asset;
        }
        assets
    }

    /// Amounts for all input notes in this operation.
    pub fn amounts_in(&self) -> [u128; N_INPUTS] {
        let mut amounts = [0u128; N_INPUTS];
        for (i, input) in self.inputs.iter().enumerate() {
            amounts[i] = input.amount;
        }
        amounts
    }

    /// Partial commitments for all input notes in this operation.
    pub fn partial_commitments_in(&self) -> [B256; N_INPUTS] {
        let mut partial_commitments = [B256::default(); N_INPUTS];
        for (i, input) in self.inputs.iter().enumerate() {
            partial_commitments[i] = input.partial_hash();
        }
        partial_commitments
    }

    /// Commitment hashes for all output commitments in this operation.
    pub fn commitment_hashes(&self) -> [B256; N_OUTPUTS] {
        let mut hashes = [B256::default(); N_OUTPUTS];
        for (i, output) in self.outputs.iter().enumerate() {
            hashes[i] = output.commitment_hash().unwrap_or_default();
        }
        hashes
    }

    /// Assets for all output commitments in this operation.
    pub fn commitment_assets(&self) -> [Address; N_OUTPUTS] {
        let mut assets = [Address::default(); N_OUTPUTS];
        for (i, output) in self.outputs.iter().enumerate() {
            assets[i] = output.commitment_asset().unwrap_or_default();
        }
        assets
    }

    /// Amounts for all output commitments in this operation.
    pub fn commitment_amounts(&self) -> [u128; N_OUTPUTS] {
        let mut amounts = [0u128; N_OUTPUTS];
        for (i, output) in self.outputs.iter().enumerate() {
            amounts[i] = output.commitment_amount().unwrap_or_default();
        }
        amounts
    }

    /// Assets for all output unshields in this operation.
    pub fn unshield_assets(&self) -> [Address; N_OUTPUTS] {
        let mut assets = [Address::default(); N_OUTPUTS];
        for (i, output) in self.outputs.iter().enumerate() {
            assets[i] = output.unshield_asset().unwrap_or_default();
        }
        assets
    }

    /// Amounts for all output unshields in this operation
    pub fn unshield_amounts(&self) -> [u128; N_OUTPUTS] {
        let mut amounts = [0u128; N_OUTPUTS];
        for (i, output) in self.outputs.iter().enumerate() {
            amounts[i] = output.unshield_amount().unwrap_or_default();
        }
        amounts
    }

    /// Partial commitments for all output commitments in this operation.
    pub fn partial_commitments_out(&self) -> [B256; N_OUTPUTS] {
        let mut partial_commitments = [B256::default(); N_OUTPUTS];
        for (i, output) in self.outputs.iter().enumerate() {
            partial_commitments[i] = output.partial_commitment().unwrap_or_default();
        }
        partial_commitments
    }
}

impl Output {
    pub fn commitment_hash(&self) -> Option<B256> {
        match self {
            Output::Commitment(c) => Some(c.hash()),
            Output::Withdraw(_) => None,
        }
    }

    pub fn commitment_asset(&self) -> Option<Address> {
        match self {
            Output::Commitment(c) => Some(c.asset),
            Output::Withdraw(_) => None,
        }
    }

    pub fn commitment_amount(&self) -> Option<u128> {
        match self {
            Output::Commitment(c) => Some(c.amount),
            Output::Withdraw(_) => None,
        }
    }

    pub fn unshield_asset(&self) -> Option<Address> {
        match self {
            Output::Commitment(_) => None,
            Output::Withdraw(w) => Some(w.asset),
        }
    }

    pub fn unshield_amount(&self) -> Option<u128> {
        match self {
            Output::Commitment(_) => None,
            Output::Withdraw(w) => Some(w.amount),
        }
    }

    pub fn partial_commitment(&self) -> Option<B256> {
        match self {
            Output::Commitment(c) => Some(c.partial_hash()),
            Output::Withdraw(_) => None,
        }
    }
}

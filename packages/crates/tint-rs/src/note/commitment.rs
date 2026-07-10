use alloy_primitives::{Address, B256};
use ark_bn254::Fr;

use crate::note::asset::AssetId;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Commitment {
    pub asset: AssetId,
    pub amount: u128,
    pub spendability_address: Address,
    pub spendability_data: B256,
    pub nullifying_priv_key: Fr,
    pub random: Fr,
    pub leaf_index: u64,
}

impl Commitment {
    pub fn new(
        asset: AssetId,
        amount: u128,
        spendability_address: Address,
        spendability_data: B256,
        nullifying_priv_key: Fr,
        random: Fr,
        leaf_index: u64,
    ) -> Self {
        Commitment {
            asset,
            amount,
            spendability_address,
            spendability_data,
            nullifying_priv_key,
            random,
            leaf_index,
        }
    }

    pub fn asset_fr(&self) -> Fr {
        self.asset.to_fr()
    }

    pub fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }

    /// keccak256(spendability_address ++ spendability_data) — public input to the circuit.
    pub fn spendability_hash(&self) -> Fr {
        // TODO: implement keccak256(spendability_address ++ spendability_data)
        Fr::from(0u128)
    }

    /// Returns the private nullifying key (nullifyingKeysIn in the circuit).
    pub fn nullifying_key(&self) -> Fr {
        Fr::from(self.nullifying_priv_key)
    }

    /// poseidon(nullifying_priv_key) — the public key committed to inside the note.
    pub fn nullifying_pub_key(&self) -> Fr {
        // TODO: implement poseidon(nullifying_priv_key)
        Fr::from(0u128)
    }

    /// poseidon(spendability_hash, nullifying_pub_key, random)
    pub fn partial_hash(&self) -> Fr {
        // TODO: implement poseidon(spendability_hash, nullifying_pub_key, random)
        Fr::from(0u128)
    }

    /// poseidon(asset, amount, partial_hash)
    pub fn hash(&self) -> Fr {
        // TODO: implement poseidon(asset, amount, partial_hash)
        Fr::from(0u128)
    }

    /// poseidon(nullifying_priv_key, leaf_index)
    pub fn nullifier(&self) -> Fr {
        // TODO: implement poseidon(nullifying_priv_key, leaf_index)
        Fr::from(0u128)
    }
}

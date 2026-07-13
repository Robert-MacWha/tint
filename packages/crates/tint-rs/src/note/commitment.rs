use alloy_primitives::{Address, B256, keccak256};
use ark_bn254::Fr;
use ark_ff::PrimeField;

use crate::{
    circuit::poseidon::poseidon_hash,
    note::{
        asset::AssetId,
        keys::{NullifierKey, NullifierPubKey},
    },
};

pub trait Commitment {
    fn asset_fr(&self) -> Fr;
    fn amount_fr(&self) -> Fr;
    fn spendability_address(&self) -> Address;
    fn spendability_data(&self) -> B256;
    fn random_fr(&self) -> Fr;
    fn nullifier_pub_key(&self) -> NullifierPubKey;

    fn hash(&self) -> Fr {
        poseidon_hash(&[self.asset_fr(), self.amount_fr(), self.partial_hash()])
    }

    fn partial_hash(&self) -> Fr {
        poseidon_hash(&[
            self.spendability_hash(),
            self.nullifier_pub_key().0,
            self.random_fr(),
        ])
    }

    fn spendability_hash(&self) -> Fr {
        let hash = keccak256(
            [
                self.spendability_address().as_slice(),
                self.spendability_data().as_slice(),
            ]
            .concat(),
        );
        Fr::from_le_bytes_mod_order(&hash.0)
    }
}

/// A receivable commitment.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct BaseCommitment {
    pub asset: AssetId,
    pub amount: u128,
    pub spendability_address: Address,
    pub spendability_data: B256,
    pub random: B256,
    pub nullifier_pub_key: NullifierPubKey,
}

/// A commitment that can be spent, including its nullifier key.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SpendableCommitment {
    pub base: BaseCommitment,
    pub nullifier_key: NullifierKey,
}

impl BaseCommitment {
    pub fn new(
        asset: AssetId,
        amount: u128,
        spendability_address: Address,
        spendability_data: B256,
        nullifier_pub_key: NullifierPubKey,
        random: B256,
    ) -> Self {
        BaseCommitment {
            asset,
            amount,
            spendability_address,
            spendability_data,
            nullifier_pub_key,
            random,
        }
    }
}

impl SpendableCommitment {
    pub fn new(base: BaseCommitment, nullifier_key: NullifierKey) -> Self {
        SpendableCommitment {
            base,
            nullifier_key,
        }
    }

    pub fn nullifier(&self) -> Fr {
        poseidon_hash(&[self.nullifier_key.0, self.base.hash()])
    }
}

impl Commitment for BaseCommitment {
    fn asset_fr(&self) -> Fr {
        Fr::from(self.asset)
    }

    fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }

    fn spendability_address(&self) -> Address {
        self.spendability_address
    }

    fn spendability_data(&self) -> B256 {
        self.spendability_data
    }

    fn random_fr(&self) -> Fr {
        Fr::from_le_bytes_mod_order(&self.random.0)
    }

    fn nullifier_pub_key(&self) -> NullifierPubKey {
        self.nullifier_pub_key.clone()
    }
}

impl Commitment for SpendableCommitment {
    fn asset_fr(&self) -> Fr {
        self.base.asset_fr()
    }

    fn amount_fr(&self) -> Fr {
        self.base.amount_fr()
    }

    fn spendability_address(&self) -> Address {
        self.base.spendability_address()
    }

    fn spendability_data(&self) -> B256 {
        self.base.spendability_data()
    }

    fn random_fr(&self) -> Fr {
        self.base.random_fr()
    }

    fn nullifier_pub_key(&self) -> NullifierPubKey {
        self.base.nullifier_pub_key()
    }
}

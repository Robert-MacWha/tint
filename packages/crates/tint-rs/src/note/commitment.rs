use alloy_primitives::{Address, B256, keccak256};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};

use crate::{
    account::keys::{EncryptionPubKey, NullifierKey, NullifierPubKey},
    circuit::poseidon2::{poseidon2_compress, poseidon2_hash},
    note::asset::AssetId,
};

pub trait Commitment {
    fn asset_fr(&self) -> Fr;
    fn amount_fr(&self) -> Fr;
    fn spendability_address(&self) -> Address;
    fn spendability_data(&self) -> B256;
    fn random_fr(&self) -> Fr;
    fn nullifier_pub_key(&self) -> NullifierPubKey;

    fn hash(&self) -> Fr {
        poseidon2_hash(&[self.asset_fr(), self.amount_fr(), self.partial_hash()])
    }

    fn partial_hash(&self) -> Fr {
        poseidon2_compress(&[
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
        Fr::from_be_bytes_mod_order(&hash.0)
    }
}

/// A receivable commitment.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseCommitment {
    pub asset: AssetId,
    pub amount: u128,
    pub spendability_address: Address,
    pub spendability_data: B256,
    pub random: B256,
    pub nullifier_pub_key: NullifierPubKey,
}

/// A commitment that can be spent, including its nullifier key.
#[derive(Default, Debug, Clone)]
pub struct SpendableCommitment {
    pub base: BaseCommitment,
    pub nullifier_key: NullifierKey,
    pub encryption_pub_key: EncryptionPubKey,
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

    pub fn as_spendable(
        &self,
        nullifier_key: NullifierKey,
        encryption_pub_key: EncryptionPubKey,
    ) -> SpendableCommitment {
        SpendableCommitment {
            base: self.clone(),
            nullifier_key,
            encryption_pub_key,
        }
    }

    pub fn nullifier(&self, nullifier_key: &NullifierKey) -> Fr {
        poseidon2_compress(&[nullifier_key.0, self.hash()])
    }
}

impl SpendableCommitment {
    pub fn new(
        base: BaseCommitment,
        nullifier_key: NullifierKey,
        encryption_pub_key: EncryptionPubKey,
    ) -> Self {
        SpendableCommitment {
            base,
            nullifier_key,
            encryption_pub_key,
        }
    }

    pub fn nullifier(&self) -> Fr {
        self.base.nullifier(&self.nullifier_key)
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

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn commitment_hash() {
        let base_commitment = BaseCommitment::new(
            AssetId::from(Address::new([1; 20])),
            100,
            Address::new([2; 20]),
            B256::new([3; 32]),
            NullifierPubKey::default(),
            B256::new([5; 32]),
        );

        let spendable_commitment = SpendableCommitment::new(
            base_commitment.clone(),
            NullifierKey::default(),
            EncryptionPubKey::default(),
        );

        assert_eq!(base_commitment.hash(), spendable_commitment.hash());
        assert_snapshot!(base_commitment.hash().to_string(), @"8732020446209299713566750912052193069337875022379129151798010424441753885630");
    }
}

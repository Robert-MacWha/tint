use alloy_primitives::{Address, B256, Bytes};
use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

use crate::{
    account::{
        keys::{EncryptionPubKey, NullifierKey, NullifierPubKey},
        spendability_hash,
    },
    circuit::poseidon2::{poseidon2_compress, poseidon2_hash},
    indexer::{address_to_fr, b256_to_fr},
    note::asset::AssetId,
};

pub trait Commitment {
    fn asset_fr(&self) -> Fr;
    fn amount_fr(&self) -> Fr;
    fn random_fr(&self) -> Fr;
    fn spendability_hash(&self) -> Fr;
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
}

/// A receivable commitment.
#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseCommitment {
    pub asset: AssetId,
    pub amount: u128,
    #[serde_as(as = "crate::serde::fr::FrAsBytes")]
    pub spendability_hash: Fr,
    pub random: B256,
    pub nullifier_pub_key: NullifierPubKey,
}

/// A commitment that can be spent, including its nullifier key.
#[derive(Default, Debug, Clone)]
pub struct SpendableCommitment {
    pub base: BaseCommitment,
    pub nullifier_key: NullifierKey,
    pub spendability_address: Address,
    pub spendability_witness: B256,
    pub spendability_input: Bytes,

    pub encryption_pub_key: EncryptionPubKey,
}

impl BaseCommitment {
    pub fn new(
        asset: AssetId,
        amount: u128,
        spendability_hash: Fr,
        nullifier_pub_key: NullifierPubKey,
        random: B256,
    ) -> Self {
        BaseCommitment {
            asset,
            amount,
            spendability_hash,
            nullifier_pub_key,
            random,
        }
    }

    pub fn as_spendable(
        &self,
        nullifier_key: NullifierKey,
        spendability_address: Address,
        spendability_witness: B256,
        spendability_input: Bytes,
        encryption_pub_key: EncryptionPubKey,
    ) -> SpendableCommitment {
        SpendableCommitment::new(
            self.asset,
            self.amount,
            nullifier_key,
            spendability_address,
            spendability_witness,
            spendability_input,
            encryption_pub_key,
            self.random,
        )
    }

    pub fn nullifier(&self, nullifier_key: &NullifierKey) -> Fr {
        poseidon2_compress(&[nullifier_key.0, self.hash()])
    }
}

impl SpendableCommitment {
    pub fn new(
        asset: AssetId,
        amount: u128,
        nullifier_key: NullifierKey,
        spendability_address: Address,
        spendability_witness: B256,
        spendability_input: Bytes,
        encryption_pub_key: EncryptionPubKey,
        random: B256,
    ) -> Self {
        let base = BaseCommitment::new(
            asset,
            amount,
            spendability_hash(spendability_address, spendability_witness),
            nullifier_key.pub_key(),
            random,
        );

        SpendableCommitment {
            base,
            nullifier_key,
            spendability_address,
            spendability_witness,
            spendability_input,
            encryption_pub_key,
        }
    }

    pub fn spendability_address_fr(&self) -> Fr {
        address_to_fr(self.spendability_address)
    }

    pub fn spendability_witness_fr(&self) -> Fr {
        b256_to_fr(self.spendability_witness)
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

    fn spendability_hash(&self) -> Fr {
        self.spendability_hash
    }

    fn random_fr(&self) -> Fr {
        b256_to_fr(self.random)
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

    fn spendability_hash(&self) -> Fr {
        self.base.spendability_hash()
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
    use alloy_primitives::Address;
    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn commitment_hash() {
        let spendable_commitment = SpendableCommitment::new(
            AssetId::from(Address::new([1; 20])),
            100,
            NullifierKey::default(),
            Address::new([2; 20]),
            B256::new([3; 32]),
            Bytes::default(),
            EncryptionPubKey::default(),
            B256::new([5; 32]),
        );
        let base_commitment = spendable_commitment.base.clone();

        assert_eq!(base_commitment.hash(), spendable_commitment.hash());
        assert_snapshot!(base_commitment.hash().to_string(), @"6798057769400104581763912905739377134824170279094112012482064995161618118540");
    }
}

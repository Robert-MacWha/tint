use alloy_primitives::{Address, B256, keccak256};
use ark_bn254::Fr;
use ark_ff::PrimeField;

use crate::{circuit::poseidon::poseidon_hash, note::asset::AssetId};

pub trait KeyMaterial: Clone + std::fmt::Debug + PartialEq + Eq {
    fn nullifying_pub_key(&self) -> Fr;
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct BaseCommitment<K: KeyMaterial> {
    pub asset: AssetId,
    pub amount: u128,
    pub spendability_address: Address,
    pub spendability_data: B256,
    pub key: K,
    pub random: Fr,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct NullifierKey(pub Fr);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct NullifierPubKey(pub Fr);

pub type Commitment = BaseCommitment<NullifierPubKey>;
pub type SpendableCommitment = BaseCommitment<NullifierKey>;

impl<K: KeyMaterial> BaseCommitment<K> {
    pub fn new(
        asset: AssetId,
        amount: u128,
        spendability_address: Address,
        spendability_data: B256,
        key: K,
        random: Fr,
    ) -> Self {
        BaseCommitment {
            asset,
            amount,
            spendability_address,
            spendability_data,
            key,
            random,
        }
    }

    pub fn asset_fr(&self) -> Fr {
        self.asset.to_fr()
    }

    pub fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }

    pub fn nullifying_pub_key(&self) -> Fr {
        self.key.nullifying_pub_key()
    }

    pub fn hash(&self) -> Fr {
        poseidon_hash(&[
            self.asset_fr().clone(),
            self.amount_fr().clone(),
            self.partial_hash(),
        ])
    }

    pub fn partial_hash(&self) -> Fr {
        poseidon_hash(&[
            self.spendability_hash(),
            self.nullifying_pub_key(),
            self.random.clone(),
        ])
    }

    pub fn spendability_hash(&self) -> Fr {
        let hash = keccak256(
            [
                self.spendability_address.as_slice(),
                self.spendability_data.as_slice(),
            ]
            .concat(),
        );
        Fr::from_le_bytes_mod_order(&hash.0)
    }
}

impl BaseCommitment<NullifierKey> {
    pub fn nullifier(&self) -> Fr {
        poseidon_hash(&[self.key.0, self.hash()])
    }
}

impl KeyMaterial for NullifierKey {
    fn nullifying_pub_key(&self) -> Fr {
        poseidon_hash(&[self.0.clone()])
    }
}

impl KeyMaterial for NullifierPubKey {
    fn nullifying_pub_key(&self) -> Fr {
        self.0.clone()
    }
}

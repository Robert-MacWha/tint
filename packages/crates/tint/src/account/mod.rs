use alloy_primitives::{Address, B256};
use ark_bn254::Fr;

use crate::{
    account::{keys::Keys, receiver::Receiver},
    circuit::poseidon2::poseidon2_compress,
    fr::b256_to_fr,
};

pub mod keys;
pub mod receiver;

#[derive(Clone, Debug)]
pub struct Account {
    keys: Keys,
    pub spendability_address: Address,
    pub spendability_witness: B256,
}

impl Account {
    pub fn new(keys: Keys, spendability_address: Address, spendability_witness: B256) -> Self {
        Self {
            keys,
            spendability_address,
            spendability_witness,
        }
    }

    pub fn receiver(&self) -> Receiver {
        Receiver::new(
            self.keys.nullifier_pub_key(),
            self.keys.encryption_pub_key(),
            self.spendability_hash(),
        )
    }

    pub fn address(&self) -> Vec<u8> {
        self.receiver().address()
    }

    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    pub fn spendability_hash(&self) -> Fr {
        spendability_hash(self.spendability_address, self.spendability_witness)
    }
}

pub fn spendability_hash(address: Address, witness: B256) -> Fr {
    let address = b256_to_fr(address.into_word());
    let witness = b256_to_fr(witness);
    poseidon2_compress(&[address, witness])
}

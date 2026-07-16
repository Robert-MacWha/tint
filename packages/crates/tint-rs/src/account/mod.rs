use alloy_primitives::{Address, B256};
use ark_ff::{BigInteger, PrimeField};

use crate::account::{keys::Keys, receiver::Receiver};

pub mod keys;
pub mod receiver;

#[derive(Clone, Debug)]
pub struct Account {
    keys: Keys,
    spendability_address: Address,
    spendability_data: B256,
}

impl Account {
    pub fn new(keys: Keys, spendability_address: Address, spendability_data: B256) -> Self {
        Self {
            keys,
            spendability_address,
            spendability_data,
        }
    }

    pub fn address(&self) -> Vec<u8> {
        let mut address = Vec::new();
        address.extend_from_slice(self.keys.encryption_pub_key().0.as_bytes());
        address.extend_from_slice(&self.keys.nullifier_pub_key().0.into_bigint().to_bytes_be());
        address.extend_from_slice(self.spendability_address.as_slice());
        address.extend_from_slice(self.spendability_data.as_slice());
        address
    }

    pub fn receiver(&self) -> Receiver {
        Receiver::new(
            self.keys.nullifier_pub_key(),
            self.keys.encryption_pub_key(),
            self.spendability_address,
            self.spendability_data,
        )
    }

    pub fn keys(&self) -> &Keys {
        &self.keys
    }
}

use alloy_primitives::{Address, B256};

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

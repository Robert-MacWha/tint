use std::sync::Arc;

use alloy_primitives::{Address, B256};
use ark_bn254::Fr;

use crate::{
    account::{
        keys::Keys, nullifying::NullifyingAccount, receiver::Receiver, spending::SpendingAccount,
        viewing::ViewingAccount,
    },
    circuit::poseidon2::poseidon2_compress,
    fr::b256_to_fr,
};

pub mod keys;
pub mod nullifying;
pub mod receiver;
pub mod spending;
pub mod viewing;

/// A full account that can view, nullify, and authorize spending of its own
/// notes.
#[derive(Clone, Debug)]
pub struct Account {
    viewing: ViewingAccount,
    nullifying: NullifyingAccount,
    spending: Arc<dyn SpendingAccount + Send + Sync>,
}

impl Account {
    pub fn new(
        viewing: ViewingAccount,
        nullifying: NullifyingAccount,
        spending: impl SpendingAccount + Send + Sync + 'static,
    ) -> Self {
        Self {
            viewing,
            nullifying,
            spending: Arc::new(spending),
        }
    }

    pub fn from_keys(keys: Keys, spending: impl SpendingAccount + Send + Sync + 'static) -> Self {
        Self::new(
            ViewingAccount::new(keys.encryption_key),
            NullifyingAccount::new(keys.nullifier_key),
            spending,
        )
    }

    pub fn viewing(&self) -> &ViewingAccount {
        &self.viewing
    }

    pub fn nullifying(&self) -> &NullifyingAccount {
        &self.nullifying
    }

    pub fn spending(&self) -> &(dyn SpendingAccount + Send + Sync) {
        self.spending.as_ref()
    }

    pub fn receiver(&self) -> Receiver {
        Receiver::new(
            self.nullifying.pub_key(),
            self.viewing.pub_key(),
            self.spending.spendability_hash(),
        )
    }

    pub fn address(&self) -> Vec<u8> {
        self.receiver().address()
    }
}

pub fn spendability_hash(address: Address, witness: B256) -> Fr {
    let address = b256_to_fr(address.into_word());
    let witness = b256_to_fr(witness);
    poseidon2_compress(&[address, witness])
}

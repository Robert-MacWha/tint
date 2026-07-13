use alloy_primitives::{Address, B256};

use crate::note::{
    asset::AssetId,
    commitment::BaseCommitment,
    keys::{EncryptionPubKey, NullifierPubKey},
};

/// Everything a sender needs to construct a note payable to a recipient.
#[derive(Debug, Clone)]
pub struct Receiver {
    pub nullifier_pub_key: NullifierPubKey,
    pub encryption_pub_key: EncryptionPubKey,
    pub spendability_address: Address,
    pub spendability_data: B256,
}

impl Receiver {
    pub fn commitment(&self, asset: AssetId, amount: u128, random: B256) -> BaseCommitment {
        BaseCommitment::new(
            asset,
            amount,
            self.spendability_address,
            self.spendability_data,
            self.nullifier_pub_key.clone(),
            random,
        )
    }
}

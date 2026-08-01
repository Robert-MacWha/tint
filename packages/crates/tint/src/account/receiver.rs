use alloy_primitives::B256;
use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

use crate::{
    account::keys::{EncryptionPubKey, NullifierPubKey},
    note::{asset::AssetId, commitment::BaseCommitment},
};

/// Represents the data required to make a note spendable by a receiver.
#[serde_with::serde_as]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Receiver {
    pub nullifier_pub_key: NullifierPubKey,
    pub encryption_pub_key: EncryptionPubKey,
    #[serde_as(as = "crate::serde::field::FieldAsBytes")]
    pub spendability_hash: Fr,
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiverAddressError {
    #[error("invalid receiver address: {0}")]
    Serialization(#[from] postcard::Error),
}

impl Receiver {
    pub fn new(
        nullifier_pub_key: NullifierPubKey,
        encryption_pub_key: EncryptionPubKey,
        spendability_hash: Fr,
    ) -> Self {
        Self {
            nullifier_pub_key,
            encryption_pub_key,
            spendability_hash,
        }
    }

    /// Parses a [`Receiver`] from the bytes produced by [`Self::address`].
    pub fn from_address(address: &[u8]) -> Result<Self, ReceiverAddressError> {
        Ok(postcard::from_bytes(address)?)
    }

    /// Creates a new [`BaseCommitment`] spendable by this receiver.
    pub fn commitment(&self, asset: AssetId, amount: u128, random: B256) -> BaseCommitment {
        BaseCommitment::new(
            asset,
            amount,
            self.spendability_hash,
            self.nullifier_pub_key,
            random,
        )
    }

    pub fn address(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("Receiver serialization is infallible")
    }
}

#[cfg(test)]
mod tests {
    use crate::account::keys::Keys;

    use super::*;

    #[test]
    fn address_round_trips() {
        let keys = Keys::from_seed(&[7u8; 32]);
        let receiver = Receiver::new(
            keys.nullifier_pub_key(),
            keys.encryption_pub_key(),
            Fr::from(42u64),
        );

        let decoded = Receiver::from_address(&receiver.address()).unwrap();

        assert_eq!(receiver, decoded);
    }
}

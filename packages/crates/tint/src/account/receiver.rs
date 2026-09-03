use alloy_primitives::B256;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use serde::{Deserialize, Serialize};
use x25519_dalek::PublicKey;

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
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub spendability_hash: Fr,
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiverAddressError {
    #[error("invalid length")]
    InvalidLength,
}

impl Receiver {
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// Errors if the provided address cannot be parsed.
    pub fn from_address(address: &[u8]) -> Result<Self, ReceiverAddressError> {
        let nullifier_pub_key_bytes = address
            .get(..32)
            .ok_or(ReceiverAddressError::InvalidLength)?;
        let encryption_pub_key_bytes: [u8; 32] = address
            .get(32..64)
            .ok_or(ReceiverAddressError::InvalidLength)?
            .try_into()
            .map_err(|_| ReceiverAddressError::InvalidLength)?;
        let spendability_hash_bytes = address
            .get(64..96)
            .ok_or(ReceiverAddressError::InvalidLength)?;

        let nullifier_pub_key =
            NullifierPubKey(Fr::from_be_bytes_mod_order(nullifier_pub_key_bytes));
        let encryption_pub_key = EncryptionPubKey(PublicKey::from(encryption_pub_key_bytes));
        let spendability_hash = Fr::from_be_bytes_mod_order(spendability_hash_bytes);
        Ok(Self {
            nullifier_pub_key,
            encryption_pub_key,
            spendability_hash,
        })
    }

    /// Creates a new [`BaseCommitment`] spendable by this receiver.
    #[must_use]
    pub fn commitment(&self, asset: AssetId, amount: u128, random: B256) -> BaseCommitment {
        BaseCommitment::new(
            asset,
            amount,
            self.spendability_hash,
            self.nullifier_pub_key,
            random,
        )
    }

    #[must_use]
    pub fn address(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.nullifier_pub_key.0.into_bigint().to_bytes_be());
        buf.extend_from_slice(self.encryption_pub_key.0.as_bytes());
        buf.extend_from_slice(&self.spendability_hash.into_bigint().to_bytes_be());
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::keys::Keys;

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

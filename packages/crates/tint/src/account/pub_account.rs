use alloy_primitives::B256;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use serde::{Deserialize, Serialize};
use x25519_dalek::PublicKey;

use crate::{
    account::keys::{EncryptionPubKey, NullifierPubKey},
    note::{asset::AssetId, commitment::BaseCommitment},
};

/// The public part of an [`Account`](crate::account::Account).
///
/// Primarily used to send notes to another user's account.
#[serde_with::serde_as]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PubAccount {
    pub nullifier_pub_key: NullifierPubKey,
    pub encryption_pub_key: EncryptionPubKey,
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub spendability_hash: Fr,
}

#[derive(Debug, thiserror::Error)]
pub enum PubAccountBytesError {
    #[error("invalid length")]
    InvalidLength,
}

impl PubAccount {
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

    /// Parses a [`PubAccount`] from the bytes produced by [`Self::to_bytes`].
    ///
    /// # Errors
    ///
    /// Errors if `bytes` cannot be parsed.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PubAccountBytesError> {
        let nullifier_pub_key_bytes = bytes.get(..32).ok_or(PubAccountBytesError::InvalidLength)?;
        let encryption_pub_key_bytes: [u8; 32] = bytes
            .get(32..64)
            .ok_or(PubAccountBytesError::InvalidLength)?
            .try_into()
            .map_err(|_| PubAccountBytesError::InvalidLength)?;
        let spendability_hash_bytes = bytes
            .get(64..96)
            .ok_or(PubAccountBytesError::InvalidLength)?;

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

    /// Creates a new [`BaseCommitment`] spendable by this account.
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
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.nullifier_pub_key.0.into_bigint().to_bytes_be());
        buf.extend_from_slice(self.encryption_pub_key.0.as_bytes());
        buf.extend_from_slice(&self.spendability_hash.into_bigint().to_bytes_be());
        buf
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;

    use super::*;
    use crate::{account::Account, policy::Policy};

    #[test]
    fn bytes_round_trip() {
        let policy = Policy::new(Address::new([1; 20]), Fr::from(42u64));
        let pub_account = Account::from_seed(&[7u8; 32], policy).pub_account();

        let decoded = PubAccount::from_bytes(&pub_account.to_bytes()).unwrap();

        assert_eq!(pub_account, decoded);
    }
}

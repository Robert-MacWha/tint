use alloy_primitives::B256;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};

use crate::{
    account::keys::{EncryptionPubKey, NullifierPubKey},
    note::{asset::AssetId, commitment::BaseCommitment},
};

/// Represents the data required to make a note spendable by a receiver.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Receiver {
    pub nullifier_pub_key: NullifierPubKey,
    pub encryption_pub_key: EncryptionPubKey,
    pub spendability_hash: Fr,
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
        let mut address = Vec::new();
        address.extend_from_slice(self.encryption_pub_key.0.as_bytes());
        address.extend_from_slice(&self.nullifier_pub_key.0.into_bigint().to_bytes_be());
        address.extend_from_slice(&self.spendability_hash.into_bigint().to_bytes_be());
        address
    }
}

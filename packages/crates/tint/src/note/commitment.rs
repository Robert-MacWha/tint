use alloy_primitives::B256;
use ark_bn254::Fr;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use tint_poseidon2::poseidon2_compress;

use crate::{
    account::keys::{EncryptionKey, EncryptionPubKey, NullifierKey, NullifierPubKey},
    crypto::envelope::EncryptedEnvelope,
    fr::b256_to_fr,
    note::asset::AssetId,
};

/// A receivable commitment.
#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub asset: AssetId,
    pub amount: u128,
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub spendability_hash: Fr,
    pub random: B256,
    pub nullifier_pub_key: NullifierPubKey,
}

#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialCommitment {
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub spendability_hash: Fr,
    pub random: B256,
    pub nullifier_pub_key: NullifierPubKey,
}

#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum CommitmentError {
    #[error("encryption error")]
    Encryption(#[from] crate::crypto::aaed::EncryptionError),
    #[error("serialization error")]
    Serialization(#[from] postcard::Error),
}

impl Commitment {
    #[must_use]
    pub fn new(
        asset: AssetId,
        amount: u128,
        spendability_hash: Fr,
        nullifier_pub_key: NullifierPubKey,
        random: B256,
    ) -> Self {
        Commitment {
            asset,
            amount,
            spendability_hash,
            random,
            nullifier_pub_key,
        }
    }

    pub fn from_encrypted(
        encrypted: &[u8],
        my_priv: &EncryptionKey,
    ) -> Result<Self, CommitmentError> {
        let encrypted: EncryptedEnvelope = postcard::from_bytes(encrypted)?;
        let plaintext = encrypted.decrypt(my_priv)?;
        Ok(postcard::from_bytes(&plaintext)?)
    }

    #[must_use]
    pub fn from_partial(asset: AssetId, amount: u128, partial: PartialCommitment) -> Self {
        Commitment {
            asset,
            amount,
            spendability_hash: partial.spendability_hash,
            random: partial.random,
            nullifier_pub_key: partial.nullifier_pub_key,
        }
    }

    #[must_use]
    pub fn partial(&self) -> PartialCommitment {
        PartialCommitment {
            spendability_hash: self.spendability_hash,
            random: self.random,
            nullifier_pub_key: self.nullifier_pub_key,
        }
    }

    #[must_use]
    pub fn hash(&self) -> Fr {
        poseidon2_compress(&[self.asset_fr(), self.amount_fr(), self.partial_hash()])
    }

    #[must_use]
    pub fn partial_hash(&self) -> Fr {
        poseidon2_compress(&[
            self.spendability_hash,
            self.nullifier_pub_key.0,
            self.random_fr(),
        ])
    }

    #[must_use]
    pub fn asset_fr(&self) -> Fr {
        Fr::from(self.asset)
    }

    #[must_use]
    pub fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }

    #[must_use]
    pub fn random_fr(&self) -> Fr {
        b256_to_fr(self.random)
    }

    #[must_use]
    pub fn nullifier(&self, nullifier_key: &NullifierKey) -> Fr {
        poseidon2_compress(&[nullifier_key.0, self.hash()])
    }

    pub fn encrypt<R: RngCore + CryptoRng>(
        &self,
        keys: &[EncryptionPubKey],
        rng: &mut R,
    ) -> Result<Vec<u8>, CommitmentError> {
        let plaintext = postcard::to_stdvec(&self)?;
        let encrypted = EncryptedEnvelope::encrypt(&plaintext, keys, rng)?;
        Ok(postcard::to_stdvec(&encrypted)?)
    }
}

impl PartialCommitment {
    #[must_use]
    pub fn new(spendability_hash: Fr, nullifier_pub_key: NullifierPubKey, random: B256) -> Self {
        PartialCommitment {
            spendability_hash,
            random,
            nullifier_pub_key,
        }
    }

    pub fn from_encrypted(
        encrypted: &[u8],
        my_priv: &EncryptionKey,
    ) -> Result<Self, CommitmentError> {
        let encrypted: EncryptedEnvelope = postcard::from_bytes(encrypted)?;
        let plaintext = encrypted.decrypt(my_priv)?;
        Ok(postcard::from_bytes(&plaintext)?)
    }

    pub fn encrypt<R: RngCore + CryptoRng>(
        &self,
        keys: &[EncryptionPubKey],
        rng: &mut R,
    ) -> Result<Vec<u8>, CommitmentError> {
        let plaintext = postcard::to_stdvec(&self)?;
        let encrypted = EncryptedEnvelope::encrypt(&plaintext, keys, rng)?;
        Ok(postcard::to_stdvec(&encrypted)?)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use insta::assert_snapshot;

    use super::*;
    use crate::{account::Account, policy::Policy};

    #[test]
    fn commitment_hash() {
        let commitment = test_commitment();

        assert_snapshot!(commitment.hash().to_string(), @"151122391010099193331386929876946401472211150702802670594863584012381564898");
    }

    #[test]
    fn commitment_encryption_decryption() {
        let keys = Account::from_seed(&[1; 32], Policy::NONE);
        let commitment = test_commitment();

        let mut rng = rand_core::OsRng;
        let encrypted = commitment
            .encrypt(&[keys.encryption_key.public_key()], &mut rng)
            .unwrap();
        let decrypted = Commitment::from_encrypted(&encrypted, &keys.encryption_key).unwrap();

        assert_eq!(decrypted, commitment);
    }

    fn test_commitment() -> Commitment {
        Commitment::new(
            AssetId::from(Address::new([1; 20])),
            100,
            Policy::new(Address::new([2; 20]), Fr::from(3)).hash(),
            NullifierKey::default().pub_key(),
            B256::new([5; 32]),
        )
    }
}

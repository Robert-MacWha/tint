use alloy_primitives::{Address, B256, Bytes};
use ark_bn254::Fr;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::{
    account::keys::{EncryptionKey, EncryptionPubKey, NullifierKey, NullifierPubKey},
    circuit::poseidon2::poseidon2_compress,
    crypto::envelope::EncryptedEnvelope,
    fr::{address_to_fr, b256_to_fr},
    note::asset::AssetId,
    policy::Policy,
};

/// A commitment that can be spent and nullified.
#[derive(Clone, Debug)]
pub struct SpendableCommitment {
    pub inner: BaseCommitment,
    pub nullifier_key: NullifierKey,
    pub spendability_address: Address,
    pub spendability_witness: Fr,
    pub spendability_input: Bytes,
}

/// A receivable commitment.
#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseCommitment {
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

impl SpendableCommitment {
    pub fn new(
        asset: AssetId,
        amount: u128,
        nullifier_key: NullifierKey,
        spendability_address: Address,
        spendability_witness: Fr,
        spendability_input: Bytes,
        random: B256,
    ) -> Self {
        let base = BaseCommitment::new(
            asset,
            amount,
            Policy::new(spendability_address, spendability_witness).hash(),
            nullifier_key.pub_key(),
            random,
        );

        SpendableCommitment {
            inner: base,
            nullifier_key,
            spendability_address,
            spendability_witness,
            spendability_input,
        }
    }

    pub fn spendability_address_fr(&self) -> Fr {
        address_to_fr(self.spendability_address)
    }

    pub fn spendability_witness_fr(&self) -> Fr {
        self.spendability_witness
    }

    pub fn nullifier(&self) -> Fr {
        self.inner.nullifier(&self.nullifier_key)
    }
}

impl BaseCommitment {
    #[must_use]
    pub fn new(
        asset: AssetId,
        amount: u128,
        spendability_hash: Fr,
        nullifier_pub_key: NullifierPubKey,
        random: B256,
    ) -> Self {
        BaseCommitment {
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
        BaseCommitment {
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

impl Default for SpendableCommitment {
    fn default() -> Self {
        let nullifier_key = NullifierKey::default();
        let base = BaseCommitment::new(
            AssetId::default(),
            0,
            Fr::from(0),
            nullifier_key.pub_key(),
            B256::default(),
        );

        SpendableCommitment {
            inner: base,
            nullifier_key,
            spendability_address: Address::default(),
            spendability_witness: Fr::default(),
            spendability_input: Bytes::default(),
        }
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
        let spendable_commitment = SpendableCommitment::new(
            AssetId::from(Address::new([1; 20])),
            100,
            NullifierKey::default(),
            Address::new([2; 20]),
            Fr::from(3),
            Bytes::default(),
            B256::new([5; 32]),
        );
        let base_commitment = spendable_commitment.inner.clone();

        assert_eq!(base_commitment.hash(), spendable_commitment.inner.hash());
        assert_snapshot!(base_commitment.hash().to_string(), @"151122391010099193331386929876946401472211150702802670594863584012381564898");
    }

    #[test]
    fn commitment_encryption_decryption() {
        let keys = Account::from_seed(&[1; 32], Policy::NONE);

        let spendable_commitment = SpendableCommitment::new(
            AssetId::from(Address::new([1; 20])),
            100,
            NullifierKey::default(),
            Address::new([2; 20]),
            Fr::from(3),
            Bytes::default(),
            B256::new([5; 32]),
        );

        let mut rng = rand_core::OsRng;
        let encrypted = spendable_commitment
            .inner
            .encrypt(&[keys.encryption_key.public_key()], &mut rng)
            .unwrap();
        let decrypted = BaseCommitment::from_encrypted(&encrypted, &keys.encryption_key).unwrap();

        assert_eq!(decrypted, spendable_commitment.inner);
    }
}

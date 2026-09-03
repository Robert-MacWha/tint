use alloy_primitives::{Address, B256, Bytes};
use ark_bn254::Fr;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::{
    account::{
        keys::{EncryptionKey, EncryptionPubKey, NullifierKey, NullifierPubKey},
        spendability_hash,
    },
    circuit::poseidon2::poseidon2_compress,
    crypto::envelope::EncryptedEnvelope,
    fr::{address_to_fr, b256_to_fr},
    note::asset::AssetId,
};

pub trait Commitment {
    fn asset_fr(&self) -> Fr;
    fn amount_fr(&self) -> Fr;
    fn random_fr(&self) -> Fr;
    fn spendability_hash(&self) -> Fr;
    fn nullifier_pub_key(&self) -> NullifierPubKey;

    fn hash(&self) -> Fr {
        poseidon2_compress(&[self.asset_fr(), self.amount_fr(), self.partial_hash()])
    }

    fn partial_hash(&self) -> Fr {
        poseidon2_compress(&[
            self.spendability_hash(),
            self.nullifier_pub_key().0,
            self.random_fr(),
        ])
    }
}

/// A commitment that can be spent and nullified.
#[derive(Clone, Debug)]
pub struct SpendableCommitment {
    pub inner: BaseCommitment,
    pub nullifier_key: NullifierKey,
    pub spendability_address: Address,
    pub spendability_witness: Fr,
    pub spendability_input: Bytes,
}

/// A commitment that can be nullified.
#[derive(Copy, Clone, Debug)]
pub struct NullifiableCommitment {
    pub inner: BaseCommitment,
    pub nullifier_key: NullifierKey,
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
            spendability_hash(spendability_address, spendability_witness),
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

impl NullifiableCommitment {
    #[must_use]
    pub fn new(inner: BaseCommitment, nullifier_key: NullifierKey) -> Self {
        NullifiableCommitment {
            inner,
            nullifier_key,
        }
    }

    #[must_use]
    pub fn nullifier(&self) -> Fr {
        self.inner.nullifier(&self.nullifier_key)
    }

    /// Builds a [`SpendableCommitment`] carrying this note's real
    /// committed data but no resolved spendability rule.
    #[must_use]
    pub fn as_pending_spendable(&self) -> SpendableCommitment {
        SpendableCommitment {
            inner: self.inner,
            nullifier_key: self.nullifier_key,
            spendability_address: Address::default(),
            spendability_witness: Fr::default(),
            spendability_input: Bytes::default(),
        }
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

    #[must_use]
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
    pub fn nullifier(&self, nullifier_key: &NullifierKey) -> Fr {
        poseidon2_compress(&[nullifier_key.0, self.hash()])
    }

    #[must_use]
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
            nullifier_pub_key,
            random,
        }
    }

    #[must_use]
    pub fn from_encrypted(
        encrypted: &[u8],
        my_priv: &EncryptionKey,
    ) -> Result<Self, CommitmentError> {
        let encrypted: EncryptedEnvelope = postcard::from_bytes(encrypted)?;
        let plaintext = encrypted.decrypt(my_priv)?;
        Ok(postcard::from_bytes(&plaintext)?)
    }

    #[must_use]
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

impl Commitment for BaseCommitment {
    fn asset_fr(&self) -> Fr {
        Fr::from(self.asset)
    }

    fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }

    fn spendability_hash(&self) -> Fr {
        self.spendability_hash
    }

    fn random_fr(&self) -> Fr {
        b256_to_fr(self.random)
    }

    fn nullifier_pub_key(&self) -> NullifierPubKey {
        self.nullifier_pub_key
    }
}

impl Commitment for NullifiableCommitment {
    fn asset_fr(&self) -> Fr {
        self.inner.asset_fr()
    }

    fn amount_fr(&self) -> Fr {
        self.inner.amount_fr()
    }

    fn spendability_hash(&self) -> Fr {
        self.inner.spendability_hash()
    }

    fn random_fr(&self) -> Fr {
        self.inner.random_fr()
    }

    fn nullifier_pub_key(&self) -> NullifierPubKey {
        self.inner.nullifier_pub_key()
    }
}

impl Commitment for SpendableCommitment {
    fn asset_fr(&self) -> Fr {
        self.inner.asset_fr()
    }

    fn amount_fr(&self) -> Fr {
        self.inner.amount_fr()
    }

    fn spendability_hash(&self) -> Fr {
        self.inner.spendability_hash()
    }

    fn random_fr(&self) -> Fr {
        self.inner.random_fr()
    }

    fn nullifier_pub_key(&self) -> NullifierPubKey {
        self.inner.nullifier_pub_key()
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
    use crate::account::keys::Keys;

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

        assert_eq!(base_commitment.hash(), spendable_commitment.hash());
        assert_snapshot!(base_commitment.hash().to_string(), @"151122391010099193331386929876946401472211150702802670594863584012381564898");
    }

    #[test]
    fn commitment_encryption_decryption() {
        let keys = Keys::from_seed(&[1; 32]);

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
            .encrypt(&[keys.encryption_pub_key()], &mut rng)
            .unwrap();
        let decrypted = BaseCommitment::from_encrypted(&encrypted, &keys.encryption_key).unwrap();

        assert_eq!(decrypted, spendable_commitment.inner);
    }
}

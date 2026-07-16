use alloy_primitives::{Address, B256};
use ark_bn254::Fr;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::{
    account::{
        keys::{EncryptionKey, EncryptionPubKey, NullifierKey, NullifierPubKey},
        spendability_hash,
    },
    crypto::{aaed::EncryptionError, envelope::EncryptedEnvelope},
    indexer::fr_to_b256,
    note::{
        asset::AssetId,
        commitment::{BaseCommitment, Commitment, SpendableCommitment},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotePayload {
    pub asset: AssetId,
    pub amount: u128,
    pub random: B256,
    pub spendability_hash: B256,
}

#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum NotePayloadError {
    #[error("encryption error")]
    Encryption(#[from] EncryptionError),
    #[error("serialization error")]
    Serialization(#[from] postcard::Error),
}

impl NotePayload {
    pub fn new(asset: AssetId, amount: u128, random: B256, spendability_hash: B256) -> Self {
        Self {
            asset,
            amount,
            random,
            spendability_hash,
        }
    }

    pub fn from_commitment(commitment: &BaseCommitment) -> Self {
        NotePayload::new(
            commitment.asset.into(),
            commitment.amount,
            commitment.random,
            fr_to_b256(commitment.spendability_hash()),
        )
    }

    pub fn from_encrypted(
        encrypted: &[u8],
        my_priv: &EncryptionKey,
    ) -> Result<Self, NotePayloadError> {
        let encrypted: EncryptedEnvelope = postcard::from_bytes(encrypted)?;
        let plaintext = encrypted.decrypt(my_priv)?;
        Ok(postcard::from_bytes(&plaintext)?)
    }

    pub fn into_spendable_commitment(
        &self,
        nullifier_key: NullifierKey,
        spendability_address: Address,
        spendability_witness: B256,
        encryption_pub_key: EncryptionPubKey,
    ) -> SpendableCommitment {
        let spendability_hash = spendability_hash(spendability_address, spendability_witness);
        let commitment = self.into_commitment(nullifier_key.pub_key(), spendability_hash);
        commitment.as_spendable(
            nullifier_key,
            spendability_address,
            spendability_witness,
            encryption_pub_key,
        )
    }

    pub fn into_commitment(
        &self,
        nullifier_pub_key: NullifierPubKey,
        spendability_hash: Fr,
    ) -> BaseCommitment {
        BaseCommitment::new(
            self.asset,
            self.amount,
            spendability_hash,
            nullifier_pub_key,
            self.random,
        )
    }

    pub fn encrypt(
        &self,
        keys: &[EncryptionPubKey],
        rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<Vec<u8>, NotePayloadError> {
        let plaintext = postcard::to_stdvec(&self)?;
        let encrypted = EncryptedEnvelope::encrypt(&plaintext, keys, rng)?;
        Ok(postcard::to_stdvec(&encrypted)?)
    }
}

use alloy_primitives::{Address, B256};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::{
    account::keys::{EncryptionKey, EncryptionPubKey},
    crypto::{aaed::EncryptionError, envelope::EncryptedEnvelope},
    note::commitment::BaseCommitment,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotePayload {
    pub asset: Address,
    pub amount: u128,
    pub random: B256,
    pub spendability_address: Address,
    pub spendability_data: B256,
}

#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum NotePayloadError {
    #[error("encryption error")]
    Encryption(#[from] EncryptionError),
    #[error("serialization error")]
    Serialization(#[from] postcard::Error),
}

impl NotePayload {
    pub fn new(
        asset: Address,
        amount: u128,
        random: B256,
        spendability_address: Address,
        spendability_data: B256,
    ) -> Self {
        Self {
            asset,
            amount,
            random,
            spendability_address,
            spendability_data,
        }
    }

    pub fn from_commitment(commitment: &BaseCommitment) -> Self {
        NotePayload::new(
            commitment.asset.into(),
            commitment.amount,
            commitment.random,
            commitment.spendability_address,
            commitment.spendability_data,
        )
    }

    pub fn from_encrypted(
        encrypted: &[u8],
        my_priv: &EncryptionKey,
    ) -> Result<Self, NotePayloadError> {
        let encrypted: EncryptedEnvelope<2> = postcard::from_bytes(encrypted)?;
        let plaintext = encrypted.decrypt(my_priv)?;
        Ok(postcard::from_bytes(&plaintext)?)
    }

    pub fn encrypt(
        &self,
        sender_pub: EncryptionPubKey,
        receiver_pub: EncryptionPubKey,
        rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<Vec<u8>, NotePayloadError> {
        let plaintext = postcard::to_stdvec(&self)?;
        let encrypted = EncryptedEnvelope::encrypt(&plaintext, &[sender_pub, receiver_pub], rng)?;
        Ok(postcard::to_stdvec(&encrypted)?)
    }
}

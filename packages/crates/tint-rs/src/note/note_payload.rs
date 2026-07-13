use alloy_primitives::{Address, B256};
use ark_std::rand::Rng;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::note::{
    commitment::BaseCommitment,
    encryption,
    keys::{EncryptionKey, EncryptionPubKey},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotePayload {
    pub asset: Address,
    pub amount: u128,
    pub random: B256,
    pub spendability_address: Address,
    pub spendability_data: B256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedNotePayload {
    pub ephemeral_pub: [u8; 32],
    pub ct_key_sender: Vec<u8>,
    pub ct_key_receiver: Vec<u8>,
    pub ct_payload: Vec<u8>,
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
    ) -> Result<Self, encryption::EncryptionError> {
        let encrypted: EncryptedNotePayload =
            postcard::from_bytes(encrypted).map_err(|_| encryption::EncryptionError::Decryption)?;
        encrypted.decrypt(my_priv)
    }

    pub fn encrypt(
        &self,
        sender_pub: &EncryptionPubKey,
        receiver_pub: &EncryptionPubKey,
        rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<Vec<u8>, encryption::EncryptionError> {
        let bytes =
            postcard::to_stdvec(&self).map_err(|_| encryption::EncryptionError::Encryption)?;

        let note_key: [u8; 32] = rng.r#gen();
        let ct_payload = encryption::aead_encrypt(&note_key, &bytes)?;

        // one ephemeral X25519 keypair shared by both recipients
        let eph_secret = StaticSecret::random_from_rng(rng);
        let ephemeral_pub = PublicKey::from(&eph_secret);

        let wrap_for = |pk_bytes: &[u8; 32]| -> Result<Vec<u8>, encryption::EncryptionError> {
            let their_pub = PublicKey::from(*pk_bytes);
            let shared = eph_secret.diffie_hellman(&their_pub);
            let kdf_key = encryption::hkdf_derive(shared.as_bytes());
            encryption::aead_encrypt(&kdf_key, &note_key)
        };

        let encrypted = EncryptedNotePayload {
            ephemeral_pub: ephemeral_pub.to_bytes(),
            ct_key_sender: wrap_for(&sender_pub.0.to_bytes())?,
            ct_key_receiver: wrap_for(&receiver_pub.0.to_bytes())?,
            ct_payload,
        };
        postcard::to_stdvec(&encrypted).map_err(|_| encryption::EncryptionError::Encryption)
    }
}

impl EncryptedNotePayload {
    pub fn decrypt(
        &self,
        my_priv: &EncryptionKey,
    ) -> Result<NotePayload, encryption::EncryptionError> {
        self.decrypt_slot(my_priv, &self.ct_key_sender)
            .or_else(|_| self.decrypt_slot(my_priv, &self.ct_key_receiver))
    }

    fn decrypt_slot(
        &self,
        my_priv: &EncryptionKey,
        ct_key: &[u8],
    ) -> Result<NotePayload, encryption::EncryptionError> {
        let ephemeral_pub = PublicKey::from(self.ephemeral_pub);

        // recompute the same shared secret the sender derived
        let shared = my_priv.0.diffie_hellman(&ephemeral_pub);
        let kdf_key = encryption::hkdf_derive(shared.as_bytes());

        // unwrap the single-use note key, then the payload
        let note_key_bytes = encryption::aead_decrypt(&kdf_key, ct_key)?;
        let note_key: [u8; 32] = note_key_bytes
            .try_into()
            .map_err(|_| encryption::EncryptionError::Decryption)?;

        let payload_bytes = encryption::aead_decrypt(&note_key, &self.ct_payload)?;
        postcard::from_bytes(&payload_bytes).map_err(|_| encryption::EncryptionError::Decryption)
    }
}

#[cfg(test)]
mod tests {
    use ark_std::rand::thread_rng;

    pub use super::*;

    #[test]
    fn note_payload_encryption() {
        let mut rng = thread_rng();

        let asset = Address::from_slice(&[1u8; 20]);
        let amount = 42u128;
        let random = B256::from_slice(&[123u8; 32]);
        let spendability_address = Address::from_slice(&[2u8; 20]);
        let spendability_data = B256::from_slice(&[3u8; 32]);

        let payload = NotePayload::new(
            asset,
            amount,
            random,
            spendability_address,
            spendability_data,
        );

        let sender_key = EncryptionKey(StaticSecret::random_from_rng(&mut rng));
        let receiver_key = EncryptionKey(StaticSecret::random_from_rng(&mut rng));
        let other_key = EncryptionKey(StaticSecret::random_from_rng(&mut rng));

        let sender_pub = sender_key.public_key();
        let receiver_pub = receiver_key.public_key();

        let encrypted = payload
            .encrypt(&sender_pub, &receiver_pub, &mut rng)
            .unwrap();

        // Decrypt for sender
        let decrypted_sender = NotePayload::from_encrypted(&encrypted, &sender_key).unwrap();
        assert_eq!(payload, decrypted_sender);

        // Decrypt for receiver
        let decrypted_receiver = NotePayload::from_encrypted(&encrypted, &receiver_key).unwrap();
        assert_eq!(payload, decrypted_receiver);

        // Decrypt with a wrong key should fail
        let decrypted_other = NotePayload::from_encrypted(&encrypted, &other_key);
        assert!(decrypted_other.is_err());
    }
}

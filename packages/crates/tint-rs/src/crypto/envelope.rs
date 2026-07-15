use ark_std::rand::Rng;
use hkdf::Hkdf;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, SharedSecret, StaticSecret};

use crate::{
    account::keys::{EncryptionKey, EncryptionPubKey},
    crypto::aaed::{EncryptionError, aead_decrypt, aead_encrypt},
};

/// An encrypted envelope that can be decrypted by any of the recipients.
///
/// Encryption is done in two steps:
/// 1. The payload is encrypted with a random symmetric key.
/// 2. The symmetric key is encrypted for each recipient's public key.
///
/// A commitment to the plaintext is included in the envelope to ensure that the decrypted payload is valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    pub ephemeral_pub: [u8; 32],
    pub commitment: [u8; 32],
    pub ct_keys: Vec<Vec<u8>>,
    pub ct_payload: Vec<u8>,
}

const ENVELOPE_INFO: &[u8] = b"tint/envelope/v1";

impl EncryptedEnvelope {
    /// Encrypts the given plaintext for the given recipients.
    pub fn encrypt(
        plaintext: &[u8],
        keys: &[EncryptionPubKey],
        mut rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<Self, EncryptionError> {
        let payload_key: [u8; 32] = rng.r#gen();
        let commitment = Sha256::digest(&plaintext).into();
        let ct_payload = aead_encrypt(&payload_key, plaintext, &mut rng)?;

        let ephemeral_secret = StaticSecret::random_from_rng(&mut rng);
        let ephemeral_pub = PublicKey::from(&ephemeral_secret);

        let mut wrap_for = |pk: &PublicKey| -> Result<Vec<u8>, EncryptionError> {
            let shared = ephemeral_secret.diffie_hellman(pk);
            let wrap_key = kdf(&shared, ENVELOPE_INFO);
            aead_encrypt(&wrap_key, &payload_key, &mut *rng)
        };

        let ct_keys = keys
            .iter()
            .map(|key| wrap_for(&key.0))
            .collect::<Result<_, EncryptionError>>()?;

        Ok(EncryptedEnvelope {
            ephemeral_pub: ephemeral_pub.to_bytes(),
            ct_keys,
            commitment,
            ct_payload,
        })
    }

    /// Decrypts the envelope using the given private key.
    ///
    /// Returns the plaintext if decryption is successful, or an error if decryption fails.
    pub fn decrypt(&self, my_priv: &EncryptionKey) -> Result<Vec<u8>, EncryptionError> {
        for ct_key in &self.ct_keys {
            if let Ok(payload) = self.decrypt_slot(my_priv, ct_key) {
                return Ok(payload);
            }
        }
        Err(EncryptionError::Decryption)
    }

    fn decrypt_slot(
        &self,
        r#priv: &EncryptionKey,
        ct_key: &[u8],
    ) -> Result<Vec<u8>, EncryptionError> {
        let ephemeral_pub = PublicKey::from(self.ephemeral_pub);

        // recompute the same shared secret the sender derived
        let shared = r#priv.0.diffie_hellman(&ephemeral_pub);
        let wrap_key = kdf(&shared, ENVELOPE_INFO);

        let payload_key_bytes = aead_decrypt(&wrap_key, ct_key)?;
        let payload_key = payload_key_bytes
            .try_into()
            .map_err(|_| EncryptionError::Decryption)?;

        let plaintext = aead_decrypt(&payload_key, &self.ct_payload)?;

        let commitment: [u8; 32] = Sha256::digest(&plaintext).into();
        if commitment != self.commitment {
            return Err(EncryptionError::InvalidCommitment);
        }
        Ok(plaintext)
    }
}

fn kdf(shared: &SharedSecret, info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut out = [0u8; 32];
    hk.expand(info, &mut out).expect("32 bytes is valid length");
    out
}

#[cfg(test)]
mod tests {
    use crate::account::keys::Keys;

    use super::*;

    #[test]
    fn envelope_encrypt_decrypt() {
        let mut rng = ark_std::rand::thread_rng();

        let key_1 = Keys::from_seed(&[1; 32]);
        let key_2 = Keys::from_seed(&[2; 32]);
        let key_3 = Keys::from_seed(&[3; 32]);
        let pub_keys = [key_1.encryption_pub_key(), key_2.encryption_pub_key()];

        let plaintext = b"Hello, world!";
        let encrypted = EncryptedEnvelope::encrypt(plaintext, &pub_keys, &mut rng).unwrap();

        let decrypted_1 = encrypted.decrypt(&key_1.encryption_key).unwrap();
        assert_eq!(decrypted_1, plaintext);

        let decrypted_2 = encrypted.decrypt(&key_2.encryption_key).unwrap();
        assert_eq!(decrypted_2, plaintext);

        let decrypted_3 = encrypted.decrypt(&key_3.encryption_key);
        assert!(decrypted_3.is_err());
    }
}

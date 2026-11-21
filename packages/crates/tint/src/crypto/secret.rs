use std::fmt::Debug;

use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use hkdf::Hkdf;
use sha2::Sha256;

use crate::crypto::aaed::{EncryptionError, aead_decrypt, aead_encrypt};

const NOTE_KEY_INFO: &[u8] = b"tint/note-key/v1";

/// A secret that notes are encrypted under, with one key per bound commitment.
///
/// Either encapsulated to a receiver, or an account's own self-secret.
#[derive(Clone, PartialEq, Eq)]
pub struct NoteSecret([u8; 32]);

impl NoteSecret {
    #[must_use]
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Encrypts `plaintext` bound to `commitment`.
    ///
    /// `commitment` must differ for everything sealed under this secret; a
    /// note's commitment does, as its randomness is fresh.
    pub fn seal(&self, commitment: Fr, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        aead_encrypt(&self.note_key(commitment), plaintext)
    }

    /// Decrypts a ciphertext [`Self::seal`] bound to `commitment`.
    pub fn open(&self, commitment: Fr, ciphertext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        aead_decrypt(&self.note_key(commitment), ciphertext)
    }

    /// Derives the single-use key bound to `commitment`.
    #[expect(clippy::expect_used)]
    fn note_key(&self, commitment: Fr) -> [u8; 32] {
        let mut info = NOTE_KEY_INFO.to_vec();
        info.extend_from_slice(&commitment.into_bigint().to_bytes_be());
        let mut key = [0u8; 32];
        // SAFETY: 32 bytes is a valid HKDF-SHA256 output length.
        Hkdf::<Sha256>::new(None, &self.0)
            .expand(&info, &mut key)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        key
    }
}

impl Debug for NoteSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoteSecret(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open() {
        let secret = NoteSecret::new([7; 32]);

        let ciphertext = secret.seal(Fr::from(1u64), b"note").unwrap();

        assert_eq!(secret.open(Fr::from(1u64), &ciphertext).unwrap(), b"note");
    }

    #[test]
    fn open_is_bound_to_commitment() {
        let secret = NoteSecret::new([7; 32]);

        let ciphertext = secret.seal(Fr::from(1u64), b"note").unwrap();

        assert!(secret.open(Fr::from(2u64), &ciphertext).is_err());
    }
}

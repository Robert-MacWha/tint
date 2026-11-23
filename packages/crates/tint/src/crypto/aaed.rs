use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::Aead};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncryptionError {
    #[error("encryption failed")]
    Encryption,
    #[error("invalid encapsulation key")]
    InvalidKey,
    #[error("decryption failed")]
    Decryption,
}

/// Encrypts `plaintext` under a key that must never encrypt anything else.
///
/// A single-use key makes the fixed zero nonce safe and saves sending one.
pub fn aead_encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    ChaCha20Poly1305::new(key.into())
        .encrypt(&Nonce::default(), plaintext)
        .map_err(|_| EncryptionError::Encryption)
}

/// Decrypts a ciphertext produced by [`aead_encrypt`].
pub fn aead_decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    ChaCha20Poly1305::new(key.into())
        .decrypt(&Nonce::default(), ciphertext)
        .map_err(|_| EncryptionError::Decryption)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aead_encrypt_decrypt() {
        let key = [0u8; 32];
        let plaintext = b"Hello, world!";
        let ciphertext = aead_encrypt(&key, plaintext).expect("encryption failed");
        let decrypted = aead_decrypt(&key, &ciphertext).expect("decryption failed");
        assert_eq!(plaintext.to_vec(), decrypted);
    }
}

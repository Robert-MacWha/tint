use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::Aead};
use hkdf::Hkdf;
use sha2::Sha256;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncryptionError {
    #[error("encryption failed")]
    Encryption,
    #[error("decryption failed")]
    Decryption,
}

const NOTE_KEY_HKDF_INFO: &[u8] = b"tint/note-key-wrap/v1";

pub fn aead_encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    let cipher = ChaCha20Poly1305::new(key.into());
    cipher
        .encrypt(&Nonce::default(), plaintext)
        .map_err(|_| EncryptionError::Encryption)
}

pub fn aead_decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    let cipher = ChaCha20Poly1305::new(key.into());
    cipher
        .decrypt(&Nonce::default(), ciphertext)
        .map_err(|_| EncryptionError::Decryption)
}

/// Derives a 32-byte ChaCha20-Poly1305 key from a raw X25519 shared secret.
pub fn hkdf_derive(shared_secret: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, shared_secret);

    let mut okm = [0u8; 32];
    hk.expand(NOTE_KEY_HKDF_INFO, &mut okm)
        .expect("32 bytes is a valid HKDF-SHA256 output length"); // can't fail for len <= 255*32

    okm
}

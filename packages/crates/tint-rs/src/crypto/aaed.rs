use chacha20poly1305::{AeadCore, ChaCha20Poly1305, KeyInit, Nonce, aead::Aead};
use rand_core::{CryptoRng, RngCore};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncryptionError {
    #[error("encryption failed")]
    Encryption,
    #[error("invalid nonce")]
    InvalidNonce,
    #[error("decryption failed")]
    Decryption,
    #[error("invalid commitment")]
    InvalidCommitment,
}

/// Encrypts the given plaintext using the given key and a random nonce.
pub fn aead_encrypt(
    key: &[u8; 32],
    plaintext: &[u8],
    rng: &mut (impl RngCore + CryptoRng),
) -> Result<Vec<u8>, EncryptionError> {
    let cipher = ChaCha20Poly1305::new(key.into());

    let nonce = ChaCha20Poly1305::generate_nonce(rng);

    let mut ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| EncryptionError::Encryption)?;
    let mut out = nonce.to_vec();
    out.append(&mut ct);
    Ok(out)
}

/// Decrypts the given ciphertext using the given key and nonce.
pub fn aead_decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    if data.len() < 12 {
        return Err(EncryptionError::Decryption);
    }

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce_arr: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| EncryptionError::Decryption)?;
    let nonce = Nonce::from(nonce_arr);

    let cipher = ChaCha20Poly1305::new(key.into());
    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| EncryptionError::Decryption)
}

#[cfg(test)]
mod tests {
    use ark_std::rand::thread_rng;

    use super::*;

    #[test]
    fn aead_encrypt_decrypt() {
        let mut rng = thread_rng();

        let key = [0u8; 32];
        let plaintext = b"Hello, world!";
        let ciphertext = aead_encrypt(&key, plaintext, &mut rng).expect("encryption failed");
        let decrypted = aead_decrypt(&key, &ciphertext).expect("decryption failed");
        assert_eq!(plaintext.to_vec(), decrypted);
    }
}

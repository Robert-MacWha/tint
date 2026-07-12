use ark_bn254::Fr;
use ark_ff::PrimeField;
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::note::commitment::NullifierKey;

const NULLIFIER_KEY_INFO: &[u8] = b"tint/nullifier-key/v1";
const ENCRYPTION_KEY_INFO: &[u8] = b"tint/encryption-key/v1";

/// A note owner's X25519 secret key, used to decrypt notes sent to them.
#[derive(Clone)]
pub struct EncryptionSecretKey(pub StaticSecret);

/// A note owner's X25519 public key, used by senders to encrypt notes to them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncryptionPubKey(pub PublicKey);

/// The full set of keys derived from a single master seed.
#[derive(Clone)]
pub struct Keys {
    pub nullifier_key: NullifierKey,
    pub encryption_secret_key: EncryptionSecretKey,
}

impl Keys {
    /// Derives a nullifier key and an encryption keypair from a single seed,
    /// via HKDF-SHA256 with distinct domain-separation labels. This lets one
    /// master seed be shared with multiple parties (e.g. for a multisig) who
    /// can then all independently derive the same keys.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let hkdf = Hkdf::<Sha256>::new(None, seed);

        let mut nullifier_bytes = [0u8; 32];
        hkdf.expand(NULLIFIER_KEY_INFO, &mut nullifier_bytes)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        let nullifier_key = NullifierKey(Fr::from_le_bytes_mod_order(&nullifier_bytes));

        let mut encryption_bytes = [0u8; 32];
        hkdf.expand(ENCRYPTION_KEY_INFO, &mut encryption_bytes)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        let encryption_secret_key = EncryptionSecretKey(StaticSecret::from(encryption_bytes));

        Keys {
            nullifier_key,
            encryption_secret_key,
        }
    }
}

impl EncryptionSecretKey {
    pub fn public_key(&self) -> EncryptionPubKey {
        EncryptionPubKey(PublicKey::from(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expect that deriving from the same seed twice gives identical keys,
    /// and that different seeds give different keys.
    #[test]
    fn from_seed_is_deterministic_and_seed_dependent() {
        let seed_a = [1u8; 32];
        let seed_b = [2u8; 32];

        let keys_a1 = Keys::from_seed(&seed_a);
        let keys_a2 = Keys::from_seed(&seed_a);
        let keys_b = Keys::from_seed(&seed_b);

        assert_eq!(keys_a1.nullifier_key.0, keys_a2.nullifier_key.0);
        assert_eq!(
            keys_a1.encryption_secret_key.public_key(),
            keys_a2.encryption_secret_key.public_key()
        );

        assert_ne!(keys_a1.nullifier_key.0, keys_b.nullifier_key.0);
        assert_ne!(
            keys_a1.encryption_secret_key.public_key(),
            keys_b.encryption_secret_key.public_key()
        );
    }
}

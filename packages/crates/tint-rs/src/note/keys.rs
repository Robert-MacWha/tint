use ark_bn254::Fr;
use ark_ff::PrimeField;
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::circuit::poseidon2::poseidon2_hash;

const NULLIFIER_KEY_INFO: &[u8] = b"tint/nullifier-key/v1";
const ENCRYPTION_KEY_INFO: &[u8] = b"tint/encryption-key/v1";

/// The full set of keys derived from a single master seed.
#[derive(Clone)]
pub struct Keys {
    pub nullifier_key: NullifierKey,
    pub encryption_key: EncryptionKey,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct NullifierKey(pub Fr);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct NullifierPubKey(pub Fr);

#[derive(Clone)]
pub struct EncryptionKey(pub StaticSecret);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncryptionPubKey(pub PublicKey);

pub trait Nullifier: Clone + std::fmt::Debug + PartialEq + Eq {
    fn nullifying_pub_key(&self) -> Fr;
}

impl Keys {
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let hkdf = Hkdf::<Sha256>::new(None, seed);

        let mut nullifier_bytes = [0u8; 32];
        hkdf.expand(NULLIFIER_KEY_INFO, &mut nullifier_bytes)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        let nullifier_key = NullifierKey(Fr::from_le_bytes_mod_order(&nullifier_bytes));

        let mut encryption_bytes = [0u8; 32];
        hkdf.expand(ENCRYPTION_KEY_INFO, &mut encryption_bytes)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        let encryption_key = EncryptionKey(StaticSecret::from(encryption_bytes));

        Keys {
            nullifier_key,
            encryption_key,
        }
    }

    pub fn nullifier_pub_key(&self) -> NullifierPubKey {
        self.nullifier_key.pub_key()
    }

    pub fn encryption_pub_key(&self) -> EncryptionPubKey {
        self.encryption_key.public_key()
    }
}

impl NullifierKey {
    pub fn pub_key(&self) -> NullifierPubKey {
        NullifierPubKey(self.nullifying_pub_key())
    }
}

impl Nullifier for NullifierKey {
    fn nullifying_pub_key(&self) -> Fr {
        poseidon2_hash(&[self.0])
    }
}

impl Nullifier for NullifierPubKey {
    fn nullifying_pub_key(&self) -> Fr {
        self.0.clone()
    }
}

impl EncryptionKey {
    pub fn public_key(&self) -> EncryptionPubKey {
        EncryptionPubKey(PublicKey::from(&self.0))
    }
}

impl Default for EncryptionPubKey {
    fn default() -> Self {
        EncryptionPubKey(PublicKey::from([0u8; 32]))
    }
}

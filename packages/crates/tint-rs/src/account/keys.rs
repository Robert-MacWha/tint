use std::fmt::Debug;

use ark_bn254::Fr;
use ark_ff::PrimeField;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::circuit::poseidon2::poseidon2_compress;

/// The full set of keys derived from a single master seed.
#[derive(Clone, Default, Debug)]
pub struct Keys {
    pub nullifier_key: NullifierKey,
    pub encryption_key: EncryptionKey,
}

#[derive(Copy, Clone, Default)]
pub struct NullifierKey(pub Fr);

#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NullifierPubKey(#[serde_as(as = "crate::serde::fr::FrAsBytes")] pub Fr);

#[derive(Clone)]
pub struct EncryptionKey(pub StaticSecret);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EncryptionPubKey(pub PublicKey);

// Info strings used for HKDF key derivation.
const NULLIFIER_KEY_INFO: &[u8] = b"tint/nullifier-key/v1";
const ENCRYPTION_KEY_INFO: &[u8] = b"tint/encryption-key/v1";

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
        NullifierPubKey(poseidon2_compress(&[self.0, Fr::from(0)]))
    }
}

impl EncryptionKey {
    pub fn public_key(&self) -> EncryptionPubKey {
        EncryptionPubKey(PublicKey::from(&self.0))
    }
}

impl Debug for NullifierKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NullifierKey(public_key = {:x?})", self.pub_key())
    }
}

impl Debug for EncryptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EncryptionKey(public_key = {:x?})",
            self.public_key().0.as_bytes()
        )
    }
}

impl Default for EncryptionKey {
    fn default() -> Self {
        EncryptionKey(StaticSecret::from([0u8; 32]))
    }
}

impl Default for EncryptionPubKey {
    fn default() -> Self {
        EncryptionPubKey(PublicKey::from([0u8; 32]))
    }
}

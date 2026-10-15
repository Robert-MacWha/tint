use std::fmt::Debug;

use ark_bn254::Fr;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::circuit::poseidon2::poseidon2_compress;

#[derive(Copy, Clone, Default)]
pub struct NullifierKey(pub Fr);

#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NullifierPubKey(#[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")] pub Fr);

#[derive(Clone)]
pub struct EncryptionKey(pub StaticSecret);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EncryptionPubKey(pub PublicKey);

impl NullifierKey {
    #[must_use]
    pub fn pub_key(&self) -> NullifierPubKey {
        NullifierPubKey(poseidon2_compress(&[self.0, Fr::from(0)]))
    }
}

impl EncryptionKey {
    #[must_use]
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

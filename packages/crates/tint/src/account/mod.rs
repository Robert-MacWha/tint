use ark_bn254::Fr;
use ark_ff::PrimeField;
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::StaticSecret;

use crate::{
    account::{
        keys::{EncryptionKey, NullifierKey},
        pub_account::PubAccount,
    },
    policy::Policy,
};

pub mod keys;
pub mod pub_account;

/// An account that can view and spend notes under its policy.
///
/// The private counterpart of a [`PubAccount`].
#[derive(Clone, Debug)]
pub struct Account {
    pub nullifier_key: NullifierKey,
    pub encryption_key: EncryptionKey,
    pub policy: Policy,
}

// Info strings used for HKDF key derivation.
const NULLIFIER_KEY_INFO: &[u8] = b"tint/nullifier-key/v1";
const ENCRYPTION_KEY_INFO: &[u8] = b"tint/encryption-key/v1";

impl Account {
    #[must_use]
    pub fn new(nullifier_key: NullifierKey, encryption_key: EncryptionKey, policy: Policy) -> Self {
        Self {
            nullifier_key,
            encryption_key,
            policy,
        }
    }

    /// Derives an account's keys from a 32-byte seed using HKDF-SHA256.
    #[must_use]
    #[allow(clippy::expect_used)]
    #[allow(clippy::missing_panics_doc)]
    pub fn from_seed(seed: &[u8; 32], policy: Policy) -> Self {
        let hkdf = Hkdf::<Sha256>::new(None, seed);

        // SAFETY: 32 bytes is a valid HKDF-SHA256 output length, so the `expect` here
        // should never fail.
        let mut nullifier_bytes = [0u8; 32];
        hkdf.expand(NULLIFIER_KEY_INFO, &mut nullifier_bytes)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        let nullifier_key = NullifierKey(Fr::from_le_bytes_mod_order(&nullifier_bytes));

        let mut encryption_bytes = [0u8; 32];
        hkdf.expand(ENCRYPTION_KEY_INFO, &mut encryption_bytes)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        let encryption_key = EncryptionKey(StaticSecret::from(encryption_bytes));

        Self::new(nullifier_key, encryption_key, policy)
    }

    /// The public account others send notes to.
    #[must_use]
    pub fn pub_account(&self) -> PubAccount {
        PubAccount::new(
            self.nullifier_key.pub_key(),
            self.encryption_key.public_key(),
            self.policy.hash(),
        )
    }
}

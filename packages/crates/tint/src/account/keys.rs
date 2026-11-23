use std::fmt::Debug;

use ark_bn254::Fr;
use hkdf::Hkdf;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shake::{ExtendableOutput, Shake256, Update, XofReader};
use tint_poseidon2::poseidon2_compress;
use x_wing::{
    CIPHERTEXT_SIZE, DECAPSULATION_KEY_SIZE, Decapsulate, ENCAPSULATION_KEY_SIZE,
    ENCAPSULATION_RANDOMNESS_SIZE, KeyExport, TryKeyInit,
};
use x25519_dalek::{PublicKey, SharedSecret, StaticSecret};

use crate::crypto::{aaed::EncryptionError, secret::NoteSecret};

/// Length of the header carrying an X-Wing ciphertext and its view tag.
pub const HEADER_LEN: usize = 1 + CIPHERTEXT_SIZE;

/// Offset of X-Wing's X25519 component within its ciphertext and encapsulation key.
const X25519_CT_OFFSET: usize = CIPHERTEXT_SIZE - 32;
const X25519_EK_OFFSET: usize = ENCAPSULATION_KEY_SIZE - 32;

const SELF_SECRET_INFO: &[u8] = b"tint/self-secret/v1";
const VIEW_TAG_DOMAIN: &[u8] = b"tint/view-tag/v1";

#[derive(Copy, Clone, Default)]
pub struct NullifierKey(pub Fr);

#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NullifierPubKey(#[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")] pub Fr);

/// Decrypts notes sent to an account, and encrypts the account's notes to itself.
#[derive(Clone)]
pub struct EncryptionKey {
    decapsulation_key: x_wing::DecapsulationKey,
    /// X-Wing's X25519 secret, used alone to check view tags.
    view_key: StaticSecret,
    self_secret: NoteSecret,
}

/// An X-Wing (ML-KEM-768 + X25519) encapsulation key.
#[serde_with::serde_as]
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EncryptionPubKey(#[serde_as(as = "serde_with::Bytes")] pub [u8; ENCAPSULATION_KEY_SIZE]);

impl NullifierKey {
    #[must_use]
    pub fn pub_key(&self) -> NullifierPubKey {
        NullifierPubKey(poseidon2_compress(&[self.0]))
    }
}

impl EncryptionKey {
    /// Derives the key from a 32-byte X-Wing decapsulation key.
    #[must_use]
    #[expect(clippy::expect_used)]
    #[expect(clippy::missing_panics_doc)]
    pub fn from_seed(seed: [u8; DECAPSULATION_KEY_SIZE]) -> Self {
        // X-Wing expands its seed with SHAKE256: bytes 64..96 are the X25519 secret.
        let mut expanded = [0u8; 96];
        let mut shake = Shake256::default();
        shake.update(&seed);
        shake.finalize_xof().read(&mut expanded);
        let mut view_key = [0u8; 32];
        view_key.copy_from_slice(&expanded[64..]);

        let mut self_secret = [0u8; 32];
        // SAFETY: 32 bytes is a valid HKDF-SHA256 output length.
        Hkdf::<Sha256>::new(None, &seed)
            .expand(SELF_SECRET_INFO, &mut self_secret)
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        Self {
            decapsulation_key: x_wing::DecapsulationKey::from(seed),
            view_key: StaticSecret::from(view_key),
            self_secret: NoteSecret::new(self_secret),
        }
    }

    #[must_use]
    pub fn public_key(&self) -> EncryptionPubKey {
        let ek = x_wing::Decapsulator::encapsulation_key(&self.decapsulation_key);
        EncryptionPubKey(ek.to_bytes().into())
    }

    /// The secret this account encrypts notes to itself under.
    #[must_use]
    pub fn self_secret(&self) -> &NoteSecret {
        &self.self_secret
    }

    /// Recovers the secret a header encapsulates to this key.
    ///
    /// Returns `None` without decapsulating if the header's view tag doesn't
    /// match, which rejects all but ~1/256 headers addressed to other keys.
    #[must_use]
    pub fn decapsulate(&self, header: &[u8; HEADER_LEN]) -> Option<NoteSecret> {
        let (tag, ct) = header.split_first()?;
        let ct_x: [u8; 32] = ct[X25519_CT_OFFSET..].try_into().ok()?;
        let shared_x = self.view_key.diffie_hellman(&PublicKey::from(ct_x));
        if view_tag(&shared_x) != *tag {
            return None;
        }

        let ct = x_wing::Ciphertext::try_from(ct).ok()?;
        Some(NoteSecret::new(self.decapsulation_key.decapsulate(&ct).into()))
    }
}

impl EncryptionPubKey {
    /// Encapsulates a fresh secret to this key, returning it and its header.
    ///
    /// # Errors
    ///
    /// Errors if this isn't a valid X-Wing encapsulation key.
    pub fn encapsulate<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
    ) -> Result<(NoteSecret, [u8; HEADER_LEN]), EncryptionError> {
        let ek = x_wing::EncapsulationKey::new_from_slice(&self.0)
            .map_err(|_| EncryptionError::InvalidKey)?;

        //? X-Wing uses the last 32 bytes of its randomness as the ephemeral X25519
        //? secret; recomputing its DH yields the shared secret the view tag commits to.
        let mut randomness = [0u8; ENCAPSULATION_RANDOMNESS_SIZE];
        rng.fill_bytes(&mut randomness);
        let (ct, shared) = ek.encapsulate_deterministic(&randomness.into());

        let mut ephemeral_x = [0u8; 32];
        ephemeral_x.copy_from_slice(&randomness[32..]);
        let pk_x: [u8; 32] = self.0[X25519_EK_OFFSET..]
            .try_into()
            .map_err(|_| EncryptionError::InvalidKey)?;
        let shared_x = StaticSecret::from(ephemeral_x).diffie_hellman(&PublicKey::from(pk_x));

        let mut header = [0u8; HEADER_LEN];
        header[0] = view_tag(&shared_x);
        header[1..].copy_from_slice(&ct);
        Ok((NoteSecret::new(shared.into()), header))
    }
}

impl Debug for NullifierKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NullifierKey(public_key = {:x?})", self.pub_key())
    }
}

impl Debug for EncryptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EncryptionKey(public_key = {:?})", self.public_key())
    }
}

impl Debug for EncryptionPubKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EncryptionPubKey(0x{}..)", alloy_primitives::hex::encode(&self.0[..8]))
    }
}

/// The first byte of a domain-separated hash of the X25519 shared secret.
fn view_tag(shared_x: &SharedSecret) -> u8 {
    Sha256::new()
        .chain_update(VIEW_TAG_DOMAIN)
        .chain_update(shared_x.as_bytes())
        .finalize()[0]
}

#[cfg(test)]
mod tests {
    use rand_core::OsRng;

    use super::*;

    #[test]
    fn view_key_is_x_wing_x25519_secret() {
        let key = EncryptionKey::from_seed([3; 32]);

        let pk_x = PublicKey::from(&key.view_key);

        assert_eq!(pk_x.as_bytes(), &key.public_key().0[X25519_EK_OFFSET..]);
    }

    #[test]
    fn decapsulate_recovers_secret() {
        let key = EncryptionKey::from_seed([1; 32]);

        let (secret, header) = key.public_key().encapsulate(&mut OsRng).unwrap();

        assert_eq!(key.decapsulate(&header), Some(secret));
    }

    #[test]
    fn wrong_view_tag_is_rejected() {
        let key = EncryptionKey::from_seed([1; 32]);
        let (_, mut header) = key.public_key().encapsulate(&mut OsRng).unwrap();

        header[0] ^= 1;

        assert_eq!(key.decapsulate(&header), None);
    }
}

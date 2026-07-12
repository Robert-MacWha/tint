use alloy_primitives::{Address, B256};
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand_core::{CryptoRng, RngCore};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::note::{
    asset::AssetId,
    commitment::{Commitment, NullifierKey, NullifierPubKey, SpendableCommitment},
    keys::{EncryptionPubKey, EncryptionSecretKey},
};

const ENCRYPTION_KEY_INFO: &[u8] = b"tint/note-encryption/v1";
const NONCE_LEN: usize = 12;
const PUB_KEY_LEN: usize = 32;
const PAYLOAD_LEN: usize = 20 + 16 + 32 + 20 + 32; // asset + amount + random + spendability_address + spendability_data

/// Everything needed to reconstruct and later spend a note, minus the
/// recipient's own nullifying public key (which they already know).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotePayload {
    pub asset: AssetId,
    pub amount: u128,
    pub random: Fr,
    pub spendability_address: Address,
    pub spendability_data: B256,
}

impl NotePayload {
    pub fn from_commitment(commitment: &Commitment) -> Self {
        Self {
            asset: commitment.asset,
            amount: commitment.amount,
            random: commitment.random,
            spendability_address: commitment.spendability_address,
            spendability_data: commitment.spendability_data,
        }
    }

    pub fn into_commitment(self, nullifier_pub_key: NullifierPubKey) -> Commitment {
        Commitment::new(
            self.asset,
            self.amount,
            self.spendability_address,
            self.spendability_data,
            nullifier_pub_key,
            self.random,
        )
    }

    /// Reconstructs the spendable note using the recipient's own nullifier
    /// key, e.g. once a recipient has decrypted a payload addressed to them.
    pub fn into_spendable_commitment(self, nullifier_key: NullifierKey) -> SpendableCommitment {
        SpendableCommitment::new(
            self.asset,
            self.amount,
            self.spendability_address,
            self.spendability_data,
            nullifier_key,
            self.random,
        )
    }

    fn to_bytes(&self) -> [u8; PAYLOAD_LEN] {
        let mut bytes = [0u8; PAYLOAD_LEN];
        let mut offset = 0;

        bytes[offset..offset + 20].copy_from_slice(self.asset.0.as_slice());
        offset += 20;
        bytes[offset..offset + 16].copy_from_slice(&self.amount.to_le_bytes());
        offset += 16;
        bytes[offset..offset + 32].copy_from_slice(&self.random.into_bigint().to_bytes_le());
        offset += 32;
        bytes[offset..offset + 20].copy_from_slice(self.spendability_address.as_slice());
        offset += 20;
        bytes[offset..offset + 32].copy_from_slice(self.spendability_data.as_slice());

        bytes
    }

    fn from_bytes(bytes: &[u8; PAYLOAD_LEN]) -> Self {
        let mut offset = 0;

        let asset = AssetId::from(Address::from_slice(&bytes[offset..offset + 20]));
        offset += 20;
        let amount = u128::from_le_bytes(bytes[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let random = Fr::from_le_bytes_mod_order(&bytes[offset..offset + 32]);
        offset += 32;
        let spendability_address = Address::from_slice(&bytes[offset..offset + 20]);
        offset += 20;
        let spendability_data = B256::from_slice(&bytes[offset..offset + 32]);

        Self {
            asset,
            amount,
            random,
            spendability_address,
            spendability_data,
        }
    }
}

/// Encrypts a note payload for `recipient`, ECIES-style: a fresh ephemeral
/// X25519 keypair is Diffie-Hellman'd with the recipient's public key to
/// derive a ChaCha20-Poly1305 key. Returns `ephemeral_pubkey || nonce ||
/// ciphertext`, suitable for posting on-chain.
pub fn encrypt<R: RngCore + CryptoRng>(
    recipient: &EncryptionPubKey,
    payload: &NotePayload,
    rng: &mut R,
) -> Vec<u8> {
    let ephemeral_secret = StaticSecret::random_from_rng(&mut *rng);
    let ephemeral_public = PublicKey::from(&ephemeral_secret);
    let shared_secret = ephemeral_secret.diffie_hellman(&recipient.0);
    let cipher = ChaCha20Poly1305::new(&derive_key(shared_secret.as_bytes()));

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, payload.to_bytes().as_slice())
        .expect("chacha20poly1305 encryption of a bounded plaintext cannot fail");

    let mut out = Vec::with_capacity(PUB_KEY_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(ephemeral_public.as_bytes());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    out
}

/// Attempts to decrypt a note payload with `secret`. Returns `None` if the
/// blob isn't addressed to this key (or is malformed) — this is how a
/// recipient scans a stream of notes to find the ones meant for them.
pub fn decrypt(secret: &EncryptionSecretKey, blob: &[u8]) -> Option<NotePayload> {
    if blob.len() < PUB_KEY_LEN + NONCE_LEN {
        return None;
    }
    let (ephemeral_public_bytes, rest) = blob.split_at(PUB_KEY_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);

    let ephemeral_public =
        PublicKey::from(<[u8; PUB_KEY_LEN]>::try_from(ephemeral_public_bytes).ok()?);
    let shared_secret = secret.0.diffie_hellman(&ephemeral_public);
    let cipher = ChaCha20Poly1305::new(&derive_key(shared_secret.as_bytes()));

    let nonce = Nonce::from(<[u8; NONCE_LEN]>::try_from(nonce_bytes).ok()?);
    let plaintext = cipher.decrypt(&nonce, ciphertext).ok()?;
    let plaintext: [u8; PAYLOAD_LEN] = plaintext.try_into().ok()?;

    Some(NotePayload::from_bytes(&plaintext))
}

fn derive_key(shared_secret: &[u8; 32]) -> Key {
    let hkdf = Hkdf::<Sha256>::new(None, shared_secret);
    let mut key_bytes = [0u8; 32];
    hkdf.expand(ENCRYPTION_KEY_INFO, &mut key_bytes)
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    Key::from(key_bytes)
}

#[cfg(test)]
mod tests {
    use rand_core::OsRng;

    use super::*;
    use crate::note::keys::Keys;

    fn sample_payload() -> NotePayload {
        NotePayload {
            asset: AssetId::from(Address::repeat_byte(0xab)),
            amount: 1234567890,
            random: Fr::from(42u64),
            spendability_address: Address::repeat_byte(0xcd),
            spendability_data: B256::repeat_byte(0xef),
        }
    }

    /// Expect that a note encrypted for a recipient can be decrypted by them,
    /// round-tripping to the original payload.
    #[test]
    fn encrypt_decrypt_round_trips() {
        let recipient = Keys::from_seed(&[7u8; 32]);
        let payload = sample_payload();

        let blob = encrypt(
            &recipient.encryption_secret_key.public_key(),
            &payload,
            &mut OsRng,
        );
        let decrypted = decrypt(&recipient.encryption_secret_key, &blob).unwrap();

        assert_eq!(decrypted, payload);
    }

    /// Expect that a note encrypted for one recipient can't be decrypted by
    /// a different recipient's key.
    #[test]
    fn decrypt_fails_for_wrong_recipient() {
        let recipient = Keys::from_seed(&[7u8; 32]);
        let other = Keys::from_seed(&[8u8; 32]);
        let payload = sample_payload();

        let blob = encrypt(
            &recipient.encryption_secret_key.public_key(),
            &payload,
            &mut OsRng,
        );
        assert!(decrypt(&other.encryption_secret_key, &blob).is_none());
    }

    /// Expect that a commitment's payload round-trips through NotePayload.
    #[test]
    fn note_payload_commitment_round_trip() {
        let recipient = Keys::from_seed(&[7u8; 32]);
        let nullifier_pub_key = recipient.nullifier_key.pub_key();
        let commitment = Commitment::new(
            AssetId::from(Address::repeat_byte(0xab)),
            10,
            Address::repeat_byte(0xcd),
            B256::repeat_byte(0xef),
            nullifier_pub_key.clone(),
            Fr::from(99u64),
        );

        let payload = NotePayload::from_commitment(&commitment);
        let rebuilt = payload.into_commitment(nullifier_pub_key);

        assert_eq!(rebuilt, commitment);
    }

    /// Expect that a decrypted payload reconstructs into a spendable
    /// commitment whose hash matches the original (public) commitment's.
    #[test]
    fn into_spendable_commitment_hash_matches_original() {
        let recipient = Keys::from_seed(&[7u8; 32]);
        let commitment = Commitment::new(
            AssetId::from(Address::repeat_byte(0xab)),
            10,
            Address::repeat_byte(0xcd),
            B256::repeat_byte(0xef),
            recipient.nullifier_key.pub_key(),
            Fr::from(99u64),
        );

        let payload = NotePayload::from_commitment(&commitment);
        let spendable = payload.into_spendable_commitment(recipient.nullifier_key.clone());

        assert_eq!(spendable.hash(), commitment.hash());
    }
}

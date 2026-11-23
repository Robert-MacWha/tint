use alloy_primitives::B256;
use ark_bn254::Fr;
use ark_ff::AdditiveGroup;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, SerializeAs};
use tint_poseidon2::poseidon2_compress;

use crate::{
    account::keys::{EncryptionKey, EncryptionPubKey, HEADER_LEN, NullifierKey, NullifierPubKey},
    crypto::{aaed::EncryptionError, secret::NoteSecret},
    fr::b256_to_fr,
    note::asset::AssetId,
};

/// A receivable commitment.
#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub asset: AssetId,
    #[serde_as(as = "U128AsBytes")]
    pub amount: u128,
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub policy_hash: Fr,
    pub random: B256,
    pub nullifier_pub_key: NullifierPubKey,
}

#[serde_with::serde_as]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialCommitment {
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub policy_hash: Fr,
    pub random: B256,
    pub nullifier_pub_key: NullifierPubKey,
}

/// Serializes a `u128` as fixed-width big-endian bytes, so an encrypted note's
/// length doesn't reveal its amount.
struct U128AsBytes;

impl SerializeAs<u128> for U128AsBytes {
    fn serialize_as<S>(source: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        source.to_be_bytes().serialize(serializer)
    }
}

impl<'de> DeserializeAs<'de, u128> for U128AsBytes {
    fn deserialize_as<D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <[u8; 16]>::deserialize(deserializer)?;
        Ok(u128::from_be_bytes(bytes))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum CommitmentError {
    #[error("encryption error")]
    Encryption(#[from] EncryptionError),
    #[error("serialization error")]
    Serialization(#[from] postcard::Error),
}

impl Commitment {
    #[must_use]
    pub fn new(
        asset: AssetId,
        amount: u128,
        policy_hash: Fr,
        nullifier_pub_key: NullifierPubKey,
        random: B256,
    ) -> Self {
        Commitment {
            asset,
            amount,
            policy_hash,
            random,
            nullifier_pub_key,
        }
    }

    /// Decrypts a note [`Self::encrypt`] encrypted for `commitment` under `secret`.
    pub fn decrypt(
        ciphertext: &[u8],
        commitment: Fr,
        secret: &NoteSecret,
    ) -> Result<Self, CommitmentError> {
        let plaintext = secret.open(commitment, ciphertext)?;
        Ok(postcard::from_bytes(&plaintext)?)
    }

    #[must_use]
    pub fn from_partial(asset: AssetId, amount: u128, partial: PartialCommitment) -> Self {
        Commitment {
            asset,
            amount,
            policy_hash: partial.policy_hash,
            random: partial.random,
            nullifier_pub_key: partial.nullifier_pub_key,
        }
    }

    #[must_use]
    pub fn partial(&self) -> PartialCommitment {
        PartialCommitment {
            policy_hash: self.policy_hash,
            random: self.random,
            nullifier_pub_key: self.nullifier_pub_key,
        }
    }

    #[must_use]
    pub fn hash(&self) -> Fr {
        if self.amount == 0 {
            return Fr::ZERO;
        }
        poseidon2_compress(&[self.asset_fr(), self.amount_fr(), self.partial_hash()])
    }

    #[must_use]
    pub fn partial_hash(&self) -> Fr {
        poseidon2_compress(&[self.policy_hash, self.nullifier_pub_key.0, self.random_fr()])
    }

    #[must_use]
    pub fn asset_fr(&self) -> Fr {
        Fr::from(self.asset)
    }

    #[must_use]
    pub fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }

    #[must_use]
    pub fn random_fr(&self) -> Fr {
        b256_to_fr(self.random)
    }

    #[must_use]
    pub fn nullifier(&self, nullifier_key: &NullifierKey) -> Fr {
        if self.amount == 0 {
            return Fr::ZERO;
        }
        poseidon2_compress(&[nullifier_key.0, self.hash()])
    }

    /// Encrypts this note under `secret`, bound to its commitment.
    pub fn encrypt(&self, secret: &NoteSecret) -> Result<Vec<u8>, CommitmentError> {
        let plaintext = postcard::to_stdvec(&self)?;
        Ok(secret.seal(self.hash(), &plaintext)?)
    }
}

impl PartialCommitment {
    #[must_use]
    pub fn new(policy_hash: Fr, nullifier_pub_key: NullifierPubKey, random: B256) -> Self {
        PartialCommitment {
            policy_hash,
            random,
            nullifier_pub_key,
        }
    }

    /// Decrypts a partial [`Self::encrypt`] encrypted to `key`.
    pub fn decrypt(ciphertext: &[u8], key: &EncryptionKey) -> Result<Self, CommitmentError> {
        let (header, body) = ciphertext
            .split_first_chunk::<HEADER_LEN>()
            .ok_or(EncryptionError::Decryption)?;
        let secret = key.decapsulate(header).ok_or(EncryptionError::Decryption)?;
        let plaintext = secret.open(Fr::ZERO, body)?;
        Ok(postcard::from_bytes(&plaintext)?)
    }

    /// Encrypts this partial commitment to `receiver`, as a deposit does.
    ///
    /// The ciphertext is an encapsulation header followed by the sealed partial.
    /// The deposit's amount may not be known yet, so it's sealed for commitment
    /// zero; its secret is fresh and seals nothing else.
    pub fn encrypt<R: RngCore + CryptoRng>(
        &self,
        receiver: &EncryptionPubKey,
        rng: &mut R,
    ) -> Result<Vec<u8>, CommitmentError> {
        let (secret, header) = receiver.encapsulate(rng)?;
        let plaintext = postcard::to_stdvec(&self)?;
        let mut ciphertext = header.to_vec();
        ciphertext.extend(secret.seal(Fr::ZERO, &plaintext)?);
        Ok(ciphertext)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use insta::assert_snapshot;

    use super::*;
    use crate::{account::Account, policy::Policy};

    #[test]
    fn commitment_hash() {
        let commitment = test_commitment();

        assert_snapshot!(commitment.hash().to_string(), @"151122391010099193331386929876946401472211150702802670594863584012381564898");
    }

    #[test]
    fn commitment_encryption_decryption() {
        let secret = NoteSecret::new([1; 32]);
        let commitment = test_commitment();

        let encrypted = commitment.encrypt(&secret).unwrap();
        let decrypted = Commitment::decrypt(&encrypted, commitment.hash(), &secret).unwrap();

        assert_eq!(decrypted, commitment);
    }

    #[test]
    fn commitment_encryption_fixed_length() {
        let secret = NoteSecret::new([1; 32]);

        let encrypted_len = |amount| {
            let commitment = Commitment {
                amount,
                ..test_commitment()
            };
            commitment.encrypt(&secret).unwrap().len()
        };

        assert_eq!(encrypted_len(1), encrypted_len(u128::MAX));
    }

    #[test]
    fn partial_encryption_decryption() {
        let keys = Account::from_seed(&[1; 32], Policy::NONE);
        let commitment = test_commitment();

        let encrypted = commitment
            .partial()
            .encrypt(&keys.encryption_key.public_key(), &mut rand_core::OsRng)
            .unwrap();
        let decrypted = PartialCommitment::decrypt(&encrypted, &keys.encryption_key).unwrap();

        assert_eq!(decrypted, commitment.partial());
    }

    fn test_commitment() -> Commitment {
        Commitment::new(
            AssetId::from(Address::new([1; 20])),
            100,
            Policy::new(Address::new([2; 20]), Fr::from(3)).hash(),
            NullifierKey::default().pub_key(),
            B256::new([5; 32]),
        )
    }
}

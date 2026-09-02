use crate::{
    account::keys::{EncryptionKey, EncryptionPubKey},
    note::commitment::{BaseCommitment, CommitmentError, PartialCommitment},
};

/// An account that can view its own transactions.
#[derive(Clone, Debug)]
pub struct ViewingAccount {
    key: EncryptionKey,
}

impl ViewingAccount {
    #[must_use]
    pub fn new(key: EncryptionKey) -> Self {
        Self { key }
    }

    #[must_use]
    pub fn pub_key(&self) -> EncryptionPubKey {
        self.key.public_key()
    }

    /// Decrypts a note encrypted to this account.
    ///
    /// # Errors
    /// Errors if the provided encrypted note cannot be decrypted with this account's key.
    pub fn decrypt_commitment(&self, encrypted: &[u8]) -> Result<BaseCommitment, CommitmentError> {
        BaseCommitment::from_encrypted(encrypted, &self.key)
    }

    /// Decrypts a partial note encrypted to this account.
    ///
    /// # Errors
    /// Errors if the provided encrypted partial note cannot be decrypted with this account's key.
    pub fn decrypt_partial_commitment(
        &self,
        encrypted_partial: &[u8],
    ) -> Result<PartialCommitment, CommitmentError> {
        PartialCommitment::from_encrypted(encrypted_partial, &self.key)
    }
}

use crate::{
    account::keys::{EncryptionKey, EncryptionPubKey},
    note::commitment::{BaseCommitment, CommitmentError},
};

/// An account that can view its own transactions.
#[derive(Clone, Debug)]
pub struct ViewingAccount {
    key: EncryptionKey,
}

impl ViewingAccount {
    pub fn new(key: EncryptionKey) -> Self {
        Self { key }
    }

    pub fn pub_key(&self) -> EncryptionPubKey {
        self.key.public_key()
    }

    /// Decrypts a note encrypted to this account.
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<BaseCommitment, CommitmentError> {
        BaseCommitment::from_encrypted(encrypted, &self.key)
    }
}

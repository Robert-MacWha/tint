use crate::{
    account::keys::{NullifierKey, NullifierPubKey},
    note::commitment::{BaseCommitment, NullifiableCommitment},
};

/// An account that can nullify (recognize as spent) its own notes.
#[derive(Clone, Debug, Default)]
pub struct NullifyingAccount {
    key: NullifierKey,
}

impl NullifyingAccount {
    pub fn new(key: NullifierKey) -> Self {
        Self { key }
    }

    pub fn key(&self) -> &NullifierKey {
        &self.key
    }

    pub fn pub_key(&self) -> NullifierPubKey {
        self.key.pub_key()
    }

    pub fn into_nullifiable(&self, inner: BaseCommitment) -> NullifiableCommitment {
        NullifiableCommitment::new(inner, self.key)
    }
}

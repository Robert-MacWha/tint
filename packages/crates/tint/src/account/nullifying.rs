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
    #[must_use] 
    pub fn new(key: NullifierKey) -> Self {
        Self { key }
    }

    #[must_use] 
    pub fn key(&self) -> &NullifierKey {
        &self.key
    }

    #[must_use] 
    pub fn pub_key(&self) -> NullifierPubKey {
        self.key.pub_key()
    }

    #[must_use] 
    pub fn into_nullifiable(&self, inner: BaseCommitment) -> NullifiableCommitment {
        NullifiableCommitment::new(inner, self.key)
    }
}

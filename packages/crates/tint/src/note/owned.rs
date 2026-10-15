use alloy_primitives::Bytes;
use ark_bn254::Fr;

use crate::{
    account::{Account, keys::NullifierKey},
    note::commitment::Commitment,
    policy::Policy,
};

/// An owned note that can be spent.
#[derive(Clone, Debug)]
pub struct OwnedNote {
    pub note: Commitment,
    pub nullifier_key: NullifierKey,
    pub policy: Policy,
}

/// A spendable [`OwnedNote`] for a specific operation.
#[derive(Clone, Debug, Default)]
pub struct SpendableNote {
    pub note: OwnedNote,
    pub policy_input: Bytes,
}

#[derive(Debug, thiserror::Error)]
pub enum OwnedNoteError {
    #[error("policy does not match the note's policy hash")]
    PolicyMismatch,
    #[error("nullifier key does not match the note's nullifier pub key")]
    NullifierKeyMismatch,
}

impl OwnedNote {
    /// # Errors
    ///
    /// Errors if `account` cannot spend `note`.
    pub fn new(note: Commitment, account: &Account) -> Result<Self, OwnedNoteError> {
        if account.nullifier_key.pub_key() != note.nullifier_pub_key {
            return Err(OwnedNoteError::NullifierKeyMismatch);
        }
        if account.policy.hash() != note.spendability_hash {
            return Err(OwnedNoteError::PolicyMismatch);
        }

        Ok(Self {
            note,
            nullifier_key: account.nullifier_key,
            policy: account.policy,
        })
    }

    #[must_use]
    pub fn commitment(&self) -> Fr {
        self.note.hash()
    }

    #[must_use]
    pub fn nullifier(&self) -> Fr {
        self.note.nullifier(&self.nullifier_key)
    }

    /// Create a [`SpendableNote`] with the given `policy_input`.
    #[must_use]
    pub fn spendable(&self, policy_input: Bytes) -> SpendableNote {
        SpendableNote {
            note: self.clone(),
            policy_input,
        }
    }
}

impl SpendableNote {
    /// Whether this is a real input.
    ///
    /// Returns `false` if this is a padding input, represented by a note with amount 0.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.note.note.amount != 0
    }
}

impl Default for OwnedNote {
    fn default() -> Self {
        let nullifier_key = NullifierKey::default();
        Self {
            note: Commitment {
                nullifier_pub_key: nullifier_key.pub_key(),
                ..Commitment::default()
            },
            nullifier_key,
            policy: Policy::default(),
        }
    }
}

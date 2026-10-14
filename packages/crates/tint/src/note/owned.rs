use alloy_primitives::Bytes;
use ark_bn254::Fr;

use crate::{
    account::{Account, keys::NullifierKey},
    note::commitment::BaseCommitment,
    policy::Policy,
};

/// A note together with everything static needed to spend it.
///
/// Holds its own nullifier key and policy, so it stays spendable on its own
/// even if the account that received it changes policy.
///
/// The per-operation policy input is supplied separately, when authorizing an
/// [`UnsignedOperation`](crate::operation::UnsignedOperation).
#[derive(Clone, Debug)]
pub struct OwnedNote {
    pub note: BaseCommitment,
    pub nullifier_key: NullifierKey,
    pub policy: Policy,
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
    pub fn new(note: BaseCommitment, account: &Account) -> Result<Self, OwnedNoteError> {
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

    /// The note's commitment, as appended to the skew-MMR.
    #[must_use]
    pub fn commitment(&self) -> Fr {
        self.note.hash()
    }

    #[must_use]
    pub fn nullifier(&self) -> Fr {
        self.note.nullifier(&self.nullifier_key)
    }

    /// This note as a circuit input, authorized by `policy_input`.
    #[must_use]
    pub fn spendable(&self, policy_input: Bytes) -> SpendableNote {
        SpendableNote {
            note: self.clone(),
            policy_input,
        }
    }
}

impl Default for OwnedNote {
    /// A zero-value note whose nullifier pub key matches its nullifier key.
    fn default() -> Self {
        let nullifier_key = NullifierKey::default();
        Self {
            note: BaseCommitment {
                nullifier_pub_key: nullifier_key.pub_key(),
                ..BaseCommitment::default()
            },
            nullifier_key,
            policy: Policy::default(),
        }
    }
}

/// An [`OwnedNote`] together with the per-operation input its policy
/// authorizes the spend with.
#[derive(Clone, Debug, Default)]
pub struct SpendableNote {
    pub note: OwnedNote,
    pub policy_input: Bytes,
}

impl SpendableNote {
    /// Whether this is a real input rather than a padding slot.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.note.note.amount != 0
    }
}

use alloy_primitives::Bytes;
use ark_bn254::Fr;

use crate::{
    account::{Account, keys::NullifierKey},
    note::commitment::{BaseCommitment, Commitment, SpendableCommitment},
    policy::Policy,
};

/// A note together with everything static needed to spend it.
///
/// Holds its own nullifier key and policy, so it stays spendable on its own
/// even if the account that received it changes policy.
///
/// The per-operation policy input is supplied separately, when authorizing an
/// [`UnsignedOperation`](crate::op::UnsignedOperation).
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
    pub fn spendable(&self, policy_input: Bytes) -> SpendableCommitment {
        SpendableCommitment::new(
            self.note.asset,
            self.note.amount,
            self.nullifier_key,
            self.policy.address,
            self.policy.witness,
            policy_input,
            self.note.random,
        )
    }
}

use std::collections::HashSet;

use alloy_primitives::Address;
use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

use crate::{
    account::{nullifying::NullifyingAccount, receiver::Receiver, viewing::ViewingAccount},
    indexer::{b256_to_fr, syncer::Event},
    note::commitment::{BaseCommitment, NullifiableCommitment},
};

pub struct IndexedAccount {
    viewing: ViewingAccount,
    nullifying: NullifyingAccount,

    /// Set of notes owned by this account.
    notes: Vec<NullifiableCommitment>,
    /// Set of nullifiers which have been spent.
    nullifiers: HashSet<Fr>,
    /// Set of nullifiers for observed commitments. Used to determine whether a
    /// new nullifier corresponds to a note owned by this account.
    note_nullifiers: HashSet<Fr>,
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IndexedAccountState {
    pub notes: Vec<BaseCommitment>,
    #[serde_as(as = "Vec<tint_groth16::serde::field::FieldAsBytes>")]
    pub nullifiers: Vec<Fr>,
    #[serde_as(as = "Vec<tint_groth16::serde::field::FieldAsBytes>")]
    pub note_nullifiers: Vec<Fr>,
}

impl IndexedAccount {
    pub async fn new(viewing: ViewingAccount, nullifying: NullifyingAccount) -> Self {
        Self {
            viewing,
            nullifying,
            notes: Vec::new(),
            nullifiers: HashSet::new(),
            note_nullifiers: HashSet::new(),
        }
    }

    /// Returns `true` if `query` identifies this account.
    #[must_use]
    pub fn matches(&self, query: &Receiver) -> bool {
        self.nullifying.pub_key() == query.nullifier_pub_key
            && self.viewing.pub_key() == query.encryption_pub_key
    }

    #[must_use]
    pub fn notes(&self) -> Vec<&NullifiableCommitment> {
        self.notes
            .iter()
            .filter(|c| !self.nullifiers.contains(&c.nullifier()))
            .collect()
    }

    /// Apply an event to this account, storing any relevant notes and nullifiers.
    pub fn apply_event(&mut self, event: &Event) {
        match event {
            Event::Deposit(d) => {
                self.decrypt_deposit(d.asset, d.amount, &d.encryptedPartial);
            }
            Event::Committed(c) => {
                self.decrypt_commitment(&c.encryptedNote);
            }
            Event::Nullified(n) => {
                let nullifier = b256_to_fr(n.nullifier);
                if self.note_nullifiers.contains(&nullifier) {
                    self.nullifiers.insert(nullifier);
                }
            }
            Event::Withdrawn(_) => {}
        }
    }

    fn decrypt_deposit(&mut self, asset: Address, amount: u128, encrypted_partial: &[u8]) {
        let Ok(partial) = self.viewing.decrypt_partial_commitment(encrypted_partial) else {
            return;
        };
        let commitment = BaseCommitment::from_partial(asset.into(), amount, partial);

        self.insert_nullifiable_commitment(commitment);
    }

    fn decrypt_commitment(&mut self, encrypted: &[u8]) {
        let Ok(commitment) = self.viewing.decrypt_commitment(encrypted) else {
            return;
        };

        self.insert_nullifiable_commitment(commitment);
    }

    fn insert_nullifiable_commitment(&mut self, commitment: BaseCommitment) {
        let nullifiable_commitment = self.nullifying.into_nullifiable(commitment);

        self.note_nullifiers
            .insert(nullifiable_commitment.nullifier());
        self.notes.push(nullifiable_commitment);
    }
}

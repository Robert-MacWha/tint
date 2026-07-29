use std::{collections::HashSet, sync::Arc};

use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

use crate::{
    account::{nullifying::NullifyingAccount, receiver::Receiver, viewing::ViewingAccount},
    database::{Database, DatabaseError, TintDatabase},
    indexer::{b256_to_fr, syncer::Event},
    note::commitment::{BaseCommitment, NullifiableCommitment},
};

pub struct IndexedAccount {
    viewing: ViewingAccount,
    nullifying: NullifyingAccount,
    database: Arc<dyn Database>,

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
    #[serde_as(as = "Vec<crate::serde::fr::FrAsBytes>")]
    pub nullifiers: Vec<Fr>,
    #[serde_as(as = "Vec<crate::serde::fr::FrAsBytes>")]
    pub note_nullifiers: Vec<Fr>,
}

impl IndexedAccount {
    pub async fn new(
        viewing: ViewingAccount,
        nullifying: NullifyingAccount,
        database: Arc<dyn Database>,
    ) -> Result<Self, DatabaseError> {
        let state = database
            .load_indexed_account(nullifying.pub_key(), viewing.pub_key())
            .await?
            .unwrap_or_default();

        let notes = state
            .notes
            .into_iter()
            .map(|c: BaseCommitment| nullifying.into_nullifiable(c))
            .collect();

        Ok(Self {
            viewing,
            nullifying,
            database,
            notes,
            nullifiers: state.nullifiers.into_iter().collect(),
            note_nullifiers: state.note_nullifiers.into_iter().collect(),
        })
    }

    /// Returns `true` if `query` identifies this account.
    pub fn matches(&self, query: &Receiver) -> bool {
        self.nullifying.pub_key() == query.nullifier_pub_key
            && self.viewing.pub_key() == query.encryption_pub_key
    }

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
                self.decrypt_commitment(&d.encryptedNote);
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
            Event::AdvanceAggregationRing(_) => {}
        }
    }

    fn decrypt_commitment(&mut self, encrypted: &[u8]) {
        let Ok(commitment) = self.viewing.decrypt(encrypted) else {
            return;
        };

        let nullifiable_commitment = self.nullifying.into_nullifiable(commitment);

        self.note_nullifiers
            .insert(nullifiable_commitment.nullifier());
        self.notes.push(nullifiable_commitment);
    }

    pub async fn save(&self) -> Result<(), DatabaseError> {
        let state = IndexedAccountState {
            notes: self.notes.iter().map(|c| c.inner).collect(),
            nullifiers: self.nullifiers.iter().copied().collect(),
            note_nullifiers: self.note_nullifiers.iter().copied().collect(),
        };

        self.database
            .set_indexed_account(self.nullifying.pub_key(), self.viewing.pub_key(), &state)
            .await
    }
}

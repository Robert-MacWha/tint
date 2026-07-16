use std::{collections::HashSet, sync::Arc};

use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

use crate::{
    account::{Account, receiver::Receiver},
    database::{Database, DatabaseError, TintDatabase},
    indexer::{b256_to_fr, syncer::Event},
    note::{
        commitment::{BaseCommitment, SpendableCommitment},
        payload::NotePayload,
    },
};

pub struct IndexedAccount {
    account: Account,
    database: Arc<dyn Database>,

    /// Set of notes owned by this account.
    spendable_notes: Vec<SpendableCommitment>,
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
    pub async fn new(account: Account, database: Arc<dyn Database>) -> Result<Self, DatabaseError> {
        let state = database
            .load_indexed_account(&account)
            .await?
            .unwrap_or_default();

        let spendable_notes = state
            .notes
            .into_iter()
            .map(|c: BaseCommitment| {
                c.as_spendable(
                    account.keys().nullifier_key.clone(),
                    account.keys().encryption_pub_key(),
                )
            })
            .collect();

        Ok(Self {
            account,
            database,
            spendable_notes,
            nullifiers: state.nullifiers.into_iter().collect(),
            note_nullifiers: state.note_nullifiers.into_iter().collect(),
        })
    }

    pub fn receiver(&self) -> Receiver {
        self.account.receiver()
    }

    pub fn spendable_notes(&self) -> Vec<&SpendableCommitment> {
        self.spendable_notes
            .iter()
            .filter(|c| !self.nullifiers.contains(&c.nullifier()))
            .collect()
    }

    /// Apply an event to this account, storing any relevant notes and nullifiers.
    pub fn apply_event(&mut self, event: &Event) {
        match event {
            Event::Deposit(d) => {
                self.decrypt_note(&d.encryptedNote);
            }
            Event::Committed(c) => {
                self.decrypt_note(&c.encryptedNote);
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

    fn decrypt_note(&mut self, encrypted: &[u8]) {
        let note = NotePayload::from_encrypted(encrypted, &self.account.keys().encryption_key);

        let Ok(note) = note else {
            return;
        };

        let commitment = note.into_spendable_commitment(
            self.account.keys().nullifier_key.clone(),
            self.account.keys().encryption_pub_key(),
        );

        self.spendable_notes.push(commitment.clone());
        self.note_nullifiers.insert(commitment.nullifier());
    }

    pub async fn save(&self) -> Result<(), DatabaseError> {
        let state = IndexedAccountState {
            notes: self.spendable_notes.iter().map(|c| c.base).collect(),
            nullifiers: self.nullifiers.iter().copied().collect(),
            note_nullifiers: self.note_nullifiers.iter().copied().collect(),
        };

        self.database
            .set_indexed_account(&self.account, &state)
            .await
    }
}

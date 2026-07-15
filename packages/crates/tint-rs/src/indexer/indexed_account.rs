use std::collections::HashSet;

use ark_bn254::Fr;

use crate::{
    account::{Account, receiver::Receiver},
    indexer::{b256_to_fr, syncer::Event},
    note::{commitment::SpendableCommitment, payload::NotePayload},
};

pub struct IndexedAccount {
    account: Account,

    /// Set of notes owned by this account.
    notes: Vec<SpendableCommitment>,

    /// Set of nullifiers which have been spent.
    nullifiers: Vec<Fr>,

    /// Set of nullifiers for observed commitments. Used to determine whether a
    /// new nullifier corresponds to a note owned by this account.
    note_nullifiers: HashSet<Fr>,
}

impl IndexedAccount {
    pub fn new(account: Account) -> Self {
        Self {
            account,
            notes: Vec::new(),
            note_nullifiers: HashSet::new(),
            nullifiers: Vec::new(),
        }
    }

    pub fn receiver(&self) -> Receiver {
        self.account.receiver()
    }

    pub fn spendable_notes(&self) -> Vec<&SpendableCommitment> {
        self.notes
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
                    self.nullifiers.push(nullifier);
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

        self.note_nullifiers.insert(commitment.nullifier());
        self.notes.push(commitment);
    }
}

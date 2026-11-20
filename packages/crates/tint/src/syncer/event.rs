use std::{collections::HashSet, ops::Range};

use alloy_primitives::{Address, Bytes};
use alloy_rpc_types_eth::Log;
use alloy_sol_types::SolEvent;
use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

use crate::{
    abis::tint::Tint,
    account::{Account, keys::EncryptionKey},
    fr::b256_to_fr,
    note::{
        asset::AssetId,
        commitment::{Commitment, PartialCommitment},
        owned::OwnedNote,
    },
};

/// A set of contiguous events emitted by a Tint deployment within a given block range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// The Tint deployment these events were emitted from.
    pub tint: Address,
    /// The half-open block range these events cover.
    pub range: Range<u64>,
    /// The events the deployment emitted within `range` in emission order.
    pub events: Vec<SyncEvent>,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshots are for different deployments")]
    DifferentDeployment,
    #[error("snapshots leave a block gap: expected start {expected}, got {got}")]
    Gap { expected: u64, got: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncEvent {
    Deposited(Deposited),
    Committed(Committed),
    Nullified(Nullified),
    Withdrawn(Withdrawn),
}

#[serde_with::serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deposited {
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub commitment: Fr,
    pub leaf_index: u64,
    pub asset: AssetId,
    pub amount: u128,
    pub encrypted_partial: Bytes,
    pub block_number: u64,
}

#[serde_with::serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committed {
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub commitment: Fr,
    pub leaf_index: u64,
    pub encrypted_note: Bytes,
    pub block_number: u64,
}

#[serde_with::serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nullified {
    #[serde_as(as = "tint_groth16::serde::field::FieldAsBytes")]
    pub nullifier: Fr,
    pub block_number: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Withdrawn {
    pub asset: AssetId,
    pub amount: u128,
    pub recipient: Address,
    pub block_number: u64,
}

impl Snapshot {
    #[must_use]
    pub fn new(tint: Address, range: Range<u64>, events: Vec<SyncEvent>) -> Self {
        Self {
            tint,
            range,
            events,
        }
    }

    /// Returns the unspent notes `account` can spend from this snapshot.
    #[must_use]
    pub fn unspent(&self, account: &Account) -> Vec<OwnedNote> {
        let nullifiers = self.nullifiers();
        self.owned(account)
            .filter(|note| !nullifiers.contains(&note.nullifier()))
            .collect()
    }

    /// Returns the notes `account` has spent within this snapshot.
    #[must_use]
    pub fn spent(&self, account: &Account) -> Vec<OwnedNote> {
        let nullifiers = self.nullifiers();
        self.owned(account)
            .filter(|note| nullifiers.contains(&note.nullifier()))
            .collect()
    }

    /// Returns the notes decryptable by `key` within this snapshot, deduplicated by commitment.
    #[must_use]
    pub fn notes(&self, key: &EncryptionKey) -> Vec<Commitment> {
        let mut seen = HashSet::new();
        self.events
            .iter()
            .filter_map(|e| e.decrypt(key))
            .filter(|note| seen.insert(note.hash()))
            .collect()
    }

    #[must_use]
    pub fn nullifiers(&self) -> HashSet<Fr> {
        self.events
            .iter()
            .filter_map(|e| match e {
                SyncEvent::Nullified(n) => Some(n.nullifier),
                _ => None,
            })
            .collect()
    }

    /// Extends this snapshot with `next`.
    ///
    /// `next` must be for the same deployment and its block range must overlap
    /// or abut this snapshot's range. Events `next` shares with this snapshot's
    /// range are ignored.
    ///
    /// # Errors
    ///
    /// Errors if `next` is for another deployment or leaves a block gap.
    pub fn extend(&mut self, next: Snapshot) -> Result<(), SnapshotError> {
        if next.tint != self.tint {
            return Err(SnapshotError::DifferentDeployment);
        }
        if next.range.start > self.range.end {
            return Err(SnapshotError::Gap {
                expected: self.range.end,
                got: next.range.start,
            });
        }

        let end = self.range.end;
        self.events
            .extend(next.events.into_iter().filter(|e| e.block_number() >= end));
        self.range.end = end.max(next.range.end);
        Ok(())
    }

    fn owned(&self, account: &Account) -> impl Iterator<Item = OwnedNote> {
        self.notes(&account.encryption_key)
            .into_iter()
            .filter_map(|note| OwnedNote::new(note, account).ok())
    }
}

impl SyncEvent {
    /// Attempts to decode an alloy RPC log into a `SyncEvent`. Returns `None` if the log is not a
    /// known event.
    #[must_use]
    pub fn try_from_log(log: &Log) -> Option<Self> {
        let block_number = log.block_number?;
        match log.topic0() {
            Some(&Tint::Deposited::SIGNATURE_HASH) => {
                let decoded = Tint::Deposited::decode_log(&log.inner).ok()?.data;
                Some(SyncEvent::Deposited(Deposited {
                    commitment: b256_to_fr(decoded.commitment),
                    leaf_index: decoded.leafIndex,
                    asset: decoded.asset.into(),
                    amount: decoded.amount,
                    encrypted_partial: decoded.encryptedPartial,
                    block_number,
                }))
            }
            Some(&Tint::Committed::SIGNATURE_HASH) => {
                let decoded = Tint::Committed::decode_log(&log.inner).ok()?.data;
                Some(SyncEvent::Committed(Committed {
                    commitment: b256_to_fr(decoded.commitment),
                    leaf_index: decoded.leafIndex,
                    encrypted_note: decoded.encryptedNote,
                    block_number,
                }))
            }
            Some(&Tint::Nullified::SIGNATURE_HASH) => {
                let decoded = Tint::Nullified::decode_log(&log.inner).ok()?.data;
                Some(SyncEvent::Nullified(Nullified {
                    nullifier: b256_to_fr(decoded.nullifier),
                    block_number,
                }))
            }
            Some(&Tint::Withdrawn::SIGNATURE_HASH) => {
                let decoded = Tint::Withdrawn::decode_log(&log.inner).ok()?.data;
                Some(SyncEvent::Withdrawn(Withdrawn {
                    asset: decoded.asset.into(),
                    amount: decoded.amount,
                    recipient: decoded.recipient,
                    block_number,
                }))
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn block_number(&self) -> u64 {
        match self {
            SyncEvent::Deposited(d) => d.block_number,
            SyncEvent::Committed(c) => c.block_number,
            SyncEvent::Nullified(n) => n.block_number,
            SyncEvent::Withdrawn(w) => w.block_number,
        }
    }

    /// The commitment this event appends to the skew-MMR and its leaf index, if any.
    #[must_use]
    pub fn commitment(&self) -> Option<(u64, Fr)> {
        match self {
            SyncEvent::Deposited(d) => Some((d.leaf_index, d.commitment)),
            SyncEvent::Committed(c) => Some((c.leaf_index, c.commitment)),
            SyncEvent::Nullified(_) | SyncEvent::Withdrawn(_) => None,
        }
    }

    /// Decrypts the note carried by this event, if it's addressed to `key` and
    /// hashes to the event's on-chain commitment.
    #[must_use]
    pub fn decrypt(&self, key: &EncryptionKey) -> Option<Commitment> {
        let (note, commitment) = match self {
            SyncEvent::Deposited(d) => {
                let partial = PartialCommitment::from_encrypted(&d.encrypted_partial, key).ok()?;
                (
                    Commitment::from_partial(d.asset, d.amount, partial),
                    d.commitment,
                )
            }
            SyncEvent::Committed(c) => (
                Commitment::from_encrypted(&c.encrypted_note, key).ok()?,
                c.commitment,
            ),
            SyncEvent::Nullified(_) | SyncEvent::Withdrawn(_) => return None,
        };
        (note.hash() == commitment).then_some(note)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, B256};
    use ark_bn254::Fr;
    use ark_std::rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::{note::asset::AssetId, policy::Policy};

    const ASSET: AssetId = AssetId(Address::repeat_byte(0xaa));

    fn account() -> Account {
        Account::from_seed(&[7u8; 32], Policy::NONE)
    }

    fn note(account: &Account, amount: u128) -> Commitment {
        account
            .pub_account()
            .commitment(ASSET, amount, B256::repeat_byte(0x42))
    }

    fn committed(account: &Account, note: &Commitment, commitment: Fr) -> SyncEvent {
        let mut rng = StdRng::seed_from_u64(1);
        let encrypted_note = note
            .encrypt(&[account.pub_account().encryption_pub_key], &mut rng)
            .unwrap()
            .into();
        SyncEvent::Committed(Committed {
            commitment,
            leaf_index: 0,
            encrypted_note,
            block_number: 1,
        })
    }

    fn deposited(account: &Account, note: &Commitment, commitment: Fr) -> SyncEvent {
        let mut rng = StdRng::seed_from_u64(2);
        let encrypted_partial = note
            .partial()
            .encrypt(&[account.pub_account().encryption_pub_key], &mut rng)
            .unwrap()
            .into();
        SyncEvent::Deposited(Deposited {
            commitment,
            leaf_index: 0,
            asset: note.asset,
            amount: note.amount,
            encrypted_partial,
            block_number: 1,
        })
    }

    fn snapshot(events: Vec<SyncEvent>) -> Snapshot {
        Snapshot::new(Address::ZERO, 0..2, events)
    }

    #[test]
    fn matching_notes_are_found() {
        let account = account();
        let note = note(&account, 1_000);
        let snapshot = snapshot(vec![
            committed(&account, &note, note.hash()),
            deposited(&account, &note, note.hash()),
        ]);

        let unspent = snapshot.unspent(&account);
        assert_eq!(unspent.len(), 1, "the same commitment twice is one note");
        assert_eq!(unspent[0].note, note);
    }

    #[test]
    fn mismatched_committed_note_is_dropped() {
        let account = account();
        let claimed = note(&account, 1_000_000);
        let event = committed(&account, &claimed, Fr::from(12345u64));

        assert!(event.decrypt(&account.encryption_key).is_none());
        assert!(snapshot(vec![event]).unspent(&account).is_empty());
    }

    #[test]
    fn mismatched_deposited_note_is_dropped() {
        let account = account();
        let claimed = note(&account, 5_000_000);
        let event = deposited(&account, &claimed, Fr::from(999u64));

        assert!(event.decrypt(&account.encryption_key).is_none());
        assert!(snapshot(vec![event]).unspent(&account).is_empty());
    }

    #[test]
    fn zero_amount_deposit_is_dropped() {
        let account = account();
        let zero = note(&account, 0);
        let on_chain = tint_poseidon2::poseidon2_compress(&[
            zero.asset_fr(),
            zero.amount_fr(),
            zero.partial_hash(),
        ]);
        let event = deposited(&account, &zero, on_chain);

        assert!(event.decrypt(&account.encryption_key).is_none());
    }
}

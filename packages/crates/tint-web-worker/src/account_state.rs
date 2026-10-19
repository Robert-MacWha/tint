use std::collections::{BTreeMap, HashMap};

use alloy::primitives::{Address, B256, hex};
use ark_bn254::Fr;
use tint::{
    account::Account,
    note::owned::OwnedNote,
    skew_mmr::SkewMmr,
    syncer::{Snapshot, SyncEvent, Withdrawn},
};
use tint_web_protocol::{
    AccountState, Balance, Deposit, GlobalEntry, HistoryEntry, HistoryKind, NoteInfo, Withdrawal,
};

/// A note `account` owns, with where it was created.
struct Owned {
    note: OwnedNote,
    nullifier: Fr,
    leaf_index: u64,
    block: u64,
    tx: B256,
    deposited: bool,
}

/// The account's notes created and spent by one transaction, per token.
#[derive(Default)]
struct Operation<'a> {
    block: u64,
    tx: B256,
    deposited: BTreeMap<Address, u128>,
    received: BTreeMap<Address, u128>,
    spent: BTreeMap<Address, u128>,
    withdrawals: Vec<&'a Withdrawn>,
}

/// `account`'s view of `snapshot`, signing as `signer`.
pub fn account_state(
    snapshot: &Snapshot,
    mmr: &SkewMmr,
    account: &Account,
    signer: Address,
) -> AccountState {
    let owned = owned_notes(snapshot, account);
    let spent_at: HashMap<Fr, (u64, B256)> = snapshot
        .events
        .iter()
        .filter_map(|e| match e {
            SyncEvent::Nullified(n) => Some((n.nullifier, (n.block_number, n.tx_hash))),
            _ => None,
        })
        .collect();

    AccountState {
        signer,
        receiver: receiver(account),
        synced_to: snapshot.range.end,
        events: snapshot.events.len(),
        leaves: mmr.state().count(),
        balances: balances(&owned, &spent_at),
        history: history(snapshot, &owned, &spent_at),
        notes: notes(&owned, &spent_at),
        global_history: global_history(snapshot),
    }
}

/// `account`'s hex-encoded receive address.
pub fn receiver(account: &Account) -> String {
    hex::encode_prefixed(account.pub_account().to_bytes())
}

/// Every note `account` owns within `snapshot`, in creation order.
fn owned_notes(snapshot: &Snapshot, account: &Account) -> Vec<Owned> {
    snapshot
        .events
        .iter()
        .filter_map(|event| {
            let note = OwnedNote::new(event.decrypt(&account.encryption_key)?, account).ok()?;
            let (leaf_index, _) = event.commitment()?;
            Some(Owned {
                nullifier: note.nullifier(),
                note,
                leaf_index,
                block: event.block_number(),
                tx: event.tx_hash(),
                deposited: matches!(event, SyncEvent::Deposited(_)),
            })
        })
        .collect()
}

fn balances(owned: &[Owned], spent_at: &HashMap<Fr, (u64, B256)>) -> Vec<Balance> {
    let mut totals = BTreeMap::<Address, (u128, usize)>::new();
    for o in owned
        .iter()
        .filter(|o| !spent_at.contains_key(&o.nullifier))
    {
        let total = totals.entry(o.note.note.asset.into()).or_default();
        total.0 += o.note.note.amount;
        total.1 += 1;
    }
    totals
        .into_iter()
        .map(|(token, (amount, notes))| Balance {
            token,
            amount,
            notes,
        })
        .collect()
}

/// The account's operations, one entry per token moved, newest first.
fn history(
    snapshot: &Snapshot,
    owned: &[Owned],
    spent_at: &HashMap<Fr, (u64, B256)>,
) -> Vec<HistoryEntry> {
    let mut ops = HashMap::<B256, Operation>::new();
    for o in owned {
        let (token, amount) = (o.note.note.asset.into(), o.note.note.amount);
        let created = operation(&mut ops, o.block, o.tx);
        let totals = if o.deposited {
            &mut created.deposited
        } else {
            &mut created.received
        };
        *totals.entry(token).or_default() += amount;

        if let Some(&(block, tx)) = spent_at.get(&o.nullifier) {
            *operation(&mut ops, block, tx)
                .spent
                .entry(token)
                .or_default() += amount;
        }
    }
    for event in &snapshot.events {
        if let SyncEvent::Withdrawn(w) = event
            && let Some(op) = ops.get_mut(&w.tx_hash)
        {
            op.withdrawals.push(w);
        }
    }

    // Events are in chain order, so a transaction's first event orders it.
    let mut order = HashMap::new();
    for (i, event) in snapshot.events.iter().enumerate() {
        order.entry(event.tx_hash()).or_insert(i);
    }
    let mut ops: Vec<_> = ops.into_values().collect();
    ops.sort_by_key(|op| std::cmp::Reverse(order.get(&op.tx).copied()));
    ops.iter().flat_map(entries).collect()
}

fn operation<'a, 'e>(
    ops: &'a mut HashMap<B256, Operation<'e>>,
    block: u64,
    tx: B256,
) -> &'a mut Operation<'e> {
    ops.entry(tx).or_insert_with(|| Operation {
        block,
        tx,
        ..Default::default()
    })
}

/// How `op` moved each token, from the account's point of view.
fn entries(op: &Operation) -> Vec<HistoryEntry> {
    let entry = |kind, token, amount, recipient| HistoryEntry {
        block: op.block,
        tx: op.tx,
        kind,
        token,
        amount,
        recipient,
    };

    // Without spending any of the account's notes, it can only have received.
    if op.spent.is_empty() {
        let deposits = op
            .deposited
            .iter()
            .map(|(&t, &a)| (HistoryKind::Deposit, t, a));
        let receives = op
            .received
            .iter()
            .map(|(&t, &a)| (HistoryKind::Receive, t, a));
        return deposits
            .chain(receives)
            .map(|(kind, token, amount)| entry(kind, token, amount, None))
            .collect();
    }

    // The account made this operation, so its withdrawals are the account's,
    // and whatever it spent beyond its change and withdrawals was sent away.
    let mut entries: Vec<_> = op
        .withdrawals
        .iter()
        .map(|w| {
            entry(
                HistoryKind::Withdraw,
                w.asset.into(),
                w.amount,
                Some(w.recipient),
            )
        })
        .collect();
    for (&token, &spent) in &op.spent {
        let change = op.received.get(&token).copied().unwrap_or_default();
        let withdrawn: u128 = op
            .withdrawals
            .iter()
            .filter(|w| Address::from(w.asset) == token)
            .map(|w| w.amount)
            .sum();
        let sent = spent.saturating_sub(change).saturating_sub(withdrawn);
        if sent > 0 {
            entries.push(entry(HistoryKind::Send, token, sent, None));
        } else if withdrawn == 0 {
            entries.push(entry(HistoryKind::SelfTransfer, token, spent, None));
        }
    }
    entries
}

fn notes(owned: &[Owned], spent_at: &HashMap<Fr, (u64, B256)>) -> Vec<NoteInfo> {
    owned
        .iter()
        .rev()
        .map(|o| NoteInfo {
            leaf_index: o.leaf_index,
            block: o.block,
            tx: o.tx,
            token: o.note.note.asset.into(),
            amount: o.note.note.amount,
            spent_in: spent_at.get(&o.nullifier).map(|&(block, _)| block),
        })
        .collect()
}

/// Every transaction in `snapshot`, newest first.
fn global_history(snapshot: &Snapshot) -> Vec<GlobalEntry> {
    let mut entries: Vec<GlobalEntry> = Vec::new();
    for event in &snapshot.events {
        let tx = event.tx_hash();
        if entries.last().is_none_or(|e| e.tx != tx) {
            entries.push(GlobalEntry {
                block: event.block_number(),
                tx,
                nullified: 0,
                committed: 0,
                deposits: Vec::new(),
                withdrawals: Vec::new(),
            });
        }
        let Some(entry) = entries.last_mut() else {
            continue;
        };

        match event {
            SyncEvent::Deposited(d) => {
                entry.committed += 1;
                entry.deposits.push(Deposit {
                    token: d.asset.into(),
                    amount: d.amount,
                });
            }
            SyncEvent::Committed(_) => entry.committed += 1,
            SyncEvent::Nullified(_) => entry.nullified += 1,
            SyncEvent::Withdrawn(w) => entry.withdrawals.push(Withdrawal {
                token: w.asset.into(),
                amount: w.amount,
                recipient: w.recipient,
            }),
        }
    }
    entries.reverse();
    entries
}

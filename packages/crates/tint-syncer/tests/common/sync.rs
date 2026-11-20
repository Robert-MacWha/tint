#![allow(dead_code)]

use tint::{account::Account, note::asset::AssetId, skew_mmr::SkewMmr};
use tint_syncer::{SkewMmrExt, Snapshot, Syncer, rpc::RpcSyncer};

use crate::common::anvil::Instance;

/// Syncs every Tint event and builds the note set's MMR from them.
pub async fn sync(instance: &Instance) -> anyhow::Result<(Snapshot, SkewMmr)> {
    let snapshot = RpcSyncer::new(instance.provider.clone())
        .sync(*instance.tint.address(), ..)
        .await?;
    let mut mmr = SkewMmr::new();
    mmr.append_events(&snapshot.events)?;
    Ok((snapshot, mmr))
}

/// Asserts `account`'s unspent notes in `snapshot` are exactly `expected`.
///
/// `expected` is in leaf order.
pub fn assert_unspent(snapshot: &Snapshot, account: &Account, expected: &[(AssetId, u128)]) {
    let unspent: Vec<_> = snapshot
        .unspent(account)
        .iter()
        .map(|note| (note.note.asset, note.note.amount))
        .collect();
    assert_eq!(unspent, expected);
}

#![allow(dead_code)]

use tint::{account::Account, note::asset::AssetId, syncer::Snapshot};

/// Logs at `RUST_LOG` (default `info`), silencing the constraint system.
pub fn init_tracing() {
    let directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let filter = tracing_subscriber::EnvFilter::new(format!("{directives},gr1cs=off,r1cs=off"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
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

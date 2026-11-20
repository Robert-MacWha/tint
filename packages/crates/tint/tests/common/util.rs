#![allow(dead_code)]

use tint::{
    account::Account,
    note::{asset::AssetId, commitment::Commitment, owned::OwnedNote},
};

/// Logs at `RUST_LOG` (default `info`), silencing the constraint system.
pub fn init_tracing() {
    let directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let filter = tracing_subscriber::EnvFilter::new(format!("{directives},gr1cs=off,r1cs=off"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// The asset and amount of each note in `notes` that `account` can spend.
pub fn owned(notes: &[Commitment], account: &Account) -> Vec<(AssetId, u128)> {
    notes
        .iter()
        .filter(|note| OwnedNote::new(**note, account).is_ok())
        .map(|note| (note.asset, note.amount))
        .collect()
}

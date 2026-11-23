//! A minimal HTTP relay for discovering intents, its client, and a reference maker.
//!
//! See `docs/intents.md`.

use std::time::{SystemTime, UNIX_EPOCH};

pub mod client;
pub mod maker;
pub mod server;

/// The current unix time, in seconds.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

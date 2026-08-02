pub mod account;
pub mod ffi;
pub mod pubkey_hash;

/// Number of signers committed to by a note's spendability witness.
pub const N_SIGNERS: usize = 3;
/// Minimum number of valid signatures required to spend.
pub const THRESHOLD: usize = 2;

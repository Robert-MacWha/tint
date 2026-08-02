pub mod multisig;
pub mod password;

use std::io::{self, Write};

use alloy::primitives::Address;
use anyhow::Context;

/// Resolves the on-chain spendability contract address,
/// preferring `env_var` and falling back to user input.
pub(crate) fn resolve_spendability_address(env_var: &str, name: &str) -> anyhow::Result<Address> {
    if let Ok(value) = std::env::var(env_var) {
        return value
            .parse()
            .with_context(|| format!("invalid {env_var} env var"));
    }

    print!("Spendability contract address for account \"{name}\": ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    input.trim().parse().context("invalid spendability address")
}

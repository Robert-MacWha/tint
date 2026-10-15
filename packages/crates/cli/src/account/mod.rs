pub mod multisig;
pub mod password;

use std::io::{self, Write};

use alloy::primitives::{Address, Bytes};
use anyhow::Context;
use tint::{operation::UnauthorizedOperation, policy::Policy};
use tint_multisig_policy::{N_SIGNERS, THRESHOLD, policy::MultisigPolicy};
use tint_password_policy::policy::PasswordPolicy;

/// An account's spending policy, able to authorize its notes' spends.
pub enum Authorizer {
    Noop,
    Password(Box<PasswordPolicy>),
    Multisig(MultisigPolicy<N_SIGNERS, THRESHOLD>),
}

impl Authorizer {
    pub fn policy(&self) -> Policy {
        match self {
            Authorizer::Noop => Policy::NONE,
            Authorizer::Password(password) => password.policy(),
            Authorizer::Multisig(multisig) => multisig.policy(),
        }
    }

    /// Produces the policy input authorizing `op`'s inputs under this policy.
    pub fn authorize(&self, op: &UnauthorizedOperation) -> anyhow::Result<Bytes> {
        match self {
            Authorizer::Noop => Ok(Bytes::new()),
            Authorizer::Password(password) => Ok(password.authorize(op)?),
            Authorizer::Multisig(multisig) => Ok(multisig.authorize(op)?),
        }
    }
}

/// Resolves the on-chain policy contract address,
/// preferring `env_var` and falling back to user input.
pub fn resolve_policy_address(env_var: &str, name: &str) -> anyhow::Result<Address> {
    if let Ok(value) = std::env::var(env_var) {
        return value
            .parse()
            .with_context(|| format!("invalid {env_var} env var"));
    }

    print!("Policy contract address for account \"{name}\": ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    input.trim().parse().context("invalid policy address")
}

pub mod hardware_wallet;
pub mod multisig;
pub mod password;
pub mod timelock;

use alloy::primitives::{Address, B256};
use anyhow::Context;
use tint::{operation::Operation, policy::Policy};
use tint_multisig_policy::{N_SIGNERS, THRESHOLD, policy::MultisigPolicy};
use tint_password_policy::policy::PasswordPolicy;
use tint_timelock_policy::policy::TimelockPolicy;

use crate::{
    account::hardware_wallet::HardwareWallet,
    config::{SpendPolicy, StoredAccount},
};

/// Spending policy options for a new account.
#[derive(clap::Args)]
pub struct PolicyArgs {
    /// The policy the account's notes are spent under
    #[arg(long, value_enum, default_value = "none")]
    policy: PolicyKind,
    #[arg(long, env = "PASSWORD_POLICY_ADDRESS")]
    password_policy_address: Option<Address>,
    #[arg(long, env = "MULTISIG_POLICY_ADDRESS")]
    multisig_policy_address: Option<Address>,
    #[arg(long, env = "HARDWARE_WALLET_POLICY_ADDRESS")]
    hardware_wallet_policy_address: Option<Address>,
    #[arg(long, env = "TIMELOCK_POLICY_ADDRESS")]
    timelock_policy_address: Option<Address>,
    /// Unix time a timelock account's notes unlock at
    #[arg(long, required_if_eq("policy", "timelock"))]
    unlock_time: Option<u64>,
    /// Ledger Live account index signing for a hardware-wallet account
    #[arg(long, conflicts_with = "signer_key")]
    ledger_index: Option<usize>,
    /// Local private key signing for a hardware-wallet account, in place of a device
    #[arg(long)]
    signer_key: Option<B256>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum PolicyKind {
    None,
    Password,
    Multisig,
    HardwareWallet,
    Timelock,
}

/// An account's spending policy, able to authorize its notes' spends.
pub enum Authorizer {
    None,
    Password(Box<PasswordPolicy>),
    Multisig(MultisigPolicy<N_SIGNERS, THRESHOLD>),
    HardwareWallet(Box<HardwareWallet>),
    Timelock(Box<TimelockPolicy>),
}

impl Authorizer {
    /// Loads `account`'s authorizer, prompting for any secrets it needs.
    pub async fn load(account: &StoredAccount, chain_id: u64) -> anyhow::Result<Self> {
        let address = account.policy_address;
        let authorizer = match &account.spend {
            SpendPolicy::None => Authorizer::None,
            SpendPolicy::Password => password::load(address)?,
            SpendPolicy::Multisig { signers } => multisig::load(address, signers)?,
            SpendPolicy::HardwareWallet(signer) => Authorizer::HardwareWallet(Box::new(
                HardwareWallet::load(address, chain_id, signer).await?,
            )),
            SpendPolicy::Timelock { unlock_time, salt } => {
                timelock::load(address, *unlock_time, *salt)?
            }
        };

        if authorizer.policy() != account.policy() {
            anyhow::bail!("credentials don't match the account's policy (wrong password?)");
        }
        Ok(authorizer)
    }

    pub fn policy(&self) -> Policy {
        match self {
            Authorizer::None => Policy::NONE,
            Authorizer::Password(password) => password.policy(),
            Authorizer::Multisig(multisig) => multisig.policy(),
            Authorizer::HardwareWallet(wallet) => wallet.policy(),
            Authorizer::Timelock(timelock) => timelock.policy(),
        }
    }

    /// Authorizes `op`'s inputs under this policy, as of block time `now`.
    pub async fn authorize(&self, op: &mut Operation, now: u64) -> anyhow::Result<()> {
        match self {
            Authorizer::None => {}
            Authorizer::Password(password) => password.authorize(op)?,
            Authorizer::Multisig(multisig) => multisig.authorize(op)?,
            Authorizer::HardwareWallet(wallet) => wallet.authorize(op).await?,
            Authorizer::Timelock(timelock) => timelock.authorize(op, now)?,
        }
        Ok(())
    }

    /// Authorizes `op` for gas estimation only.
    ///
    /// Uses placeholder proofs where the policy has them, so only multisig and
    /// hardware-wallet accounts sign twice for a sponsored operation.
    pub async fn authorize_simulation(&self, op: &mut Operation, now: u64) -> anyhow::Result<()> {
        match self {
            Authorizer::Password(password) => password.authorize_placeholder(op)?,
            Authorizer::Timelock(timelock) => timelock.authorize_placeholder(op, now)?,
            _ => self.authorize(op, now).await?,
        }
        Ok(())
    }
}

/// Creates a new account with a random seed under the chosen policy.
pub async fn create(args: &PolicyArgs) -> anyhow::Result<StoredAccount> {
    let mut seed = B256::ZERO;
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, seed.as_mut_slice());

    let (policy, spend) = match args.policy {
        PolicyKind::None => (Policy::NONE, SpendPolicy::None),
        PolicyKind::Password => {
            let address = require(args.password_policy_address, "PASSWORD_POLICY_ADDRESS")?;
            (password::create(address)?, SpendPolicy::Password)
        }
        PolicyKind::Multisig => {
            let address = require(args.multisig_policy_address, "MULTISIG_POLICY_ADDRESS")?;
            multisig::create(address)?
        }
        PolicyKind::HardwareWallet => {
            let address = require(
                args.hardware_wallet_policy_address,
                "HARDWARE_WALLET_POLICY_ADDRESS",
            )?;
            hardware_wallet::create(address, args.ledger_index, args.signer_key).await?
        }
        PolicyKind::Timelock => {
            let address = require(args.timelock_policy_address, "TIMELOCK_POLICY_ADDRESS")?;
            let unlock_time = args.unlock_time.context("--unlock-time is required")?;
            timelock::create(address, unlock_time)?
        }
    };
    Ok(StoredAccount::new(seed, policy, spend))
}

fn require(address: Option<Address>, env_var: &str) -> anyhow::Result<Address> {
    address.with_context(|| format!("the policy contract address is required (set {env_var})"))
}

use std::path::PathBuf;

use alloy::primitives::Address;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use tint::account::{Account, keys::Keys};

const DEFAULT_SIGNALING_SERVER: &str = "wss://test.mosquitto.org:8081/mqtt";

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub tint_address: Option<Address>,
    pub signaling_server: Option<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config file {}", path.display()))?;
        Ok(serde_json::from_str(&contents)?)
    }

    /// Resolves the Tint contract address, preferring `override_address` (a CLI flag) over
    /// the config file. Errors with a helpful message if neither is set.
    pub fn resolve_tint_address(&self, override_address: Option<Address>) -> anyhow::Result<Address> {
        override_address.or(self.tint_address).ok_or_else(|| {
            anyhow::anyhow!(
                "no tint contract address set; pass --tint-address or add \"tint_address\" to {}",
                config_path().display()
            )
        })
    }

    /// Resolves the openlv signaling server, preferring `override_server` (a CLI flag) over
    /// the config file, falling back to the public test broker.
    pub fn resolve_signaling_server(&self, override_server: Option<String>) -> String {
        override_server
            .or_else(|| self.signaling_server.clone())
            .unwrap_or_else(|| DEFAULT_SIGNALING_SERVER.to_string())
    }
}

#[derive(Serialize, Deserialize)]
struct StoredAccount {
    seed: String,
}

/// Creates a new named account, failing if one with this name already exists.
pub fn create_account(name: &str) -> anyhow::Result<Account> {
    let dir = account_dir(name);
    if dir.exists() {
        anyhow::bail!("account \"{name}\" already exists at {}", dir.display());
    }
    std::fs::create_dir_all(&dir)?;

    let mut seed = [0u8; 32];
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut seed);

    let stored = StoredAccount {
        seed: alloy::primitives::hex::encode_prefixed(seed),
    };
    std::fs::write(
        account_file(name),
        serde_json::to_string_pretty(&stored)?,
    )
    .with_context(|| format!("writing account file for \"{name}\""))?;

    Ok(account_from_seed(&seed))
}

/// Loads a previously created named account.
pub fn load_account(name: &str) -> anyhow::Result<Account> {
    let path = account_file(name);
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("no account named \"{name}\" (looked for {})", path.display()))?;
    let stored: StoredAccount = serde_json::from_str(&contents)?;

    let seed_bytes = alloy::primitives::hex::decode(&stored.seed)?;
    let seed: [u8; 32] = seed_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("corrupt account file for \"{name}\": seed is not 32 bytes"))?;

    Ok(account_from_seed(&seed))
}

fn account_from_seed(seed: &[u8; 32]) -> Account {
    Account::new(
        Keys::from_seed(seed),
        Address::ZERO,
        alloy::primitives::B256::ZERO,
    )
}

fn config_path() -> PathBuf {
    base_dir().join("config.json")
}

fn account_dir(name: &str) -> PathBuf {
    base_dir().join("accounts").join(name)
}

fn account_file(name: &str) -> PathBuf {
    account_dir(name).join("account.json")
}

fn base_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME must be set")).join(".tint-cli")
}

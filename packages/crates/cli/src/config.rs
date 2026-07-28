use std::path::PathBuf;

use alloy::primitives::Address;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use tint::account::{Account, keys::Keys};

#[derive(Serialize, Deserialize)]
struct StoredAccount {
    seed: String,
}

/// Lists all local account names, returning an empty list if none exist.
pub fn list_accounts() -> anyhow::Result<Vec<String>> {
    let accounts_dir = base_dir().join("accounts");
    if !accounts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut accounts = Vec::new();
    for entry in std::fs::read_dir(accounts_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let name = entry.file_name().into_string().map_err(|_| {
            anyhow::anyhow!(
                "account directory name is not valid UTF-8: {}",
                entry.path().display()
            )
        })?;
        accounts.push(name);
    }
    Ok(accounts)
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
    std::fs::write(account_file(name), serde_json::to_string_pretty(&stored)?)
        .with_context(|| format!("writing account file for \"{name}\""))?;

    Ok(account_from_seed(&seed))
}

/// Loads a previously created named account.
pub fn load_account(name: &str) -> anyhow::Result<Account> {
    let path = account_file(name);
    let contents = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "no account named \"{name}\" (looked for {})",
            path.display()
        )
    })?;
    let stored: StoredAccount = serde_json::from_str(&contents)?;

    let seed_bytes = alloy::primitives::hex::decode(&stored.seed)?;
    let seed: [u8; 32] = seed_bytes.try_into().map_err(|_| {
        anyhow::anyhow!("corrupt account file for \"{name}\": seed is not 32 bytes")
    })?;

    Ok(account_from_seed(&seed))
}

fn account_from_seed(seed: &[u8; 32]) -> Account {
    Account::new(
        Keys::from_seed(seed),
        Address::ZERO,
        alloy::primitives::B256::ZERO,
    )
}

fn account_dir(name: &str) -> PathBuf {
    base_dir().join("accounts").join(name)
}

fn account_file(name: &str) -> PathBuf {
    account_dir(name).join("account.json")
}

fn base_dir() -> PathBuf {
    PathBuf::from(".tint-cli")
}

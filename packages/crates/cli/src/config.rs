use std::path::PathBuf;

use anyhow::Context;
use ark_bn254::Bn254;
use ark_groth16::{ProvingKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use tint::{
    account::{Account, keys::Keys, spending::NoopSpendingAccount},
    circuit::artifacts,
};

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

/// Loads the cached Groth16 proving/verifying keys from disk, generating and
/// caching them on first use.
pub fn load_or_generate_circuit_keys() -> anyhow::Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>)> {
    let pk_path = circuit_dir().join("proving_key.bin.br");
    let vk_path = circuit_dir().join("verifying_key.bin.br");

    if pk_path.exists() && vk_path.exists() {
        tracing::info!("Loading cached circuit keys...");
        let pk_bytes =
            std::fs::read(&pk_path).with_context(|| format!("reading {}", pk_path.display()))?;
        let vk_bytes =
            std::fs::read(&vk_path).with_context(|| format!("reading {}", vk_path.display()))?;
        let proving_key = artifacts::deserialize_proving_key(&pk_bytes)
            .context("deserializing cached proving key")?;
        let verifying_key = artifacts::deserialize_verifying_key(&vk_bytes)
            .context("deserializing cached verifying key")?;
        return Ok((proving_key, verifying_key));
    }

    tracing::info!("Generating circuit keys (first run)...");
    let (proving_key, verifying_key) = tint::circuit::setup_circuits()?;

    tracing::info!(
        "Caching circuit keys to {}... this may take a while",
        circuit_dir().display()
    );
    std::fs::create_dir_all(circuit_dir())?;
    let pk_bytes =
        artifacts::serialize_proving_key(&proving_key).context("serializing proving key")?;
    let vk_bytes =
        artifacts::serialize_verifying_key(&verifying_key).context("serializing verifying key")?;
    std::fs::write(&pk_path, pk_bytes).with_context(|| format!("writing {}", pk_path.display()))?;
    std::fs::write(&vk_path, vk_bytes).with_context(|| format!("writing {}", vk_path.display()))?;

    Ok((proving_key, verifying_key))
}

fn circuit_dir() -> PathBuf {
    base_dir().join("circuit")
}

fn account_from_seed(seed: &[u8; 32]) -> Account {
    Account::from_keys(Keys::from_seed(seed), NoopSpendingAccount)
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

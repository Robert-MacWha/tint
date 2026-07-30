use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use alloy::primitives::{Address, keccak256};
use anyhow::Context;
use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{ProvingKey, VerifyingKey};
use ark_relations::gr1cs::ConstraintSynthesizer;
use serde::{Deserialize, Serialize};
use tint::{
    account::{Account, keys::Keys, receiver::Receiver, spending::NoopSpendingAccount},
    circuit::{
        artifacts::{
            deserialize_proving_key, deserialize_verifying_key, serialize_proving_key,
            serialize_verifying_key,
        },
        setup_circuit,
    },
};
use tint_spendability::{
    account::PasswordSpendingAccount, circuit::password::PasswordSpendability,
};
use tracing::info;

#[derive(Serialize, Deserialize)]
struct StoredAccount {
    seed: String,
    receiver: Receiver,
    spendability: AccountSpendability,
}

#[derive(Clone, Serialize, Deserialize, clap::ValueEnum)]
pub enum AccountSpendability {
    Noop,
    Password,
}

/// Lists all local account names, returning an empty list if none exist.
pub fn list_accounts() -> anyhow::Result<Vec<String>> {
    let accounts_dir = base_dir().join("accounts");
    if !accounts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut accounts = Vec::new();
    for entry in fs::read_dir(accounts_dir)? {
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
pub fn create_account(
    name: &str,
    spendability: AccountSpendability,
    spendability_address: Option<Address>,
) -> anyhow::Result<Account> {
    let dir = account_dir(name);
    if dir.exists() {
        anyhow::bail!("account \"{name}\" already exists at {}", dir.display());
    }
    fs::create_dir_all(&dir)?;

    let mut seed = [0u8; 32];
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut seed);

    let account = account_from_seed(&seed, spendability.clone(), spendability_address, || {
        prompt_new_password(name)
    })?;

    let stored = StoredAccount {
        seed: alloy::primitives::hex::encode_prefixed(seed),
        receiver: account.receiver(),
        spendability,
    };
    fs::write(account_file(name), serde_json::to_string_pretty(&stored)?)
        .with_context(|| format!("writing account file for \"{name}\""))?;

    Ok(account)
}

/// Loads a previously created named account, prompting for its password if
/// it uses password spendability.
pub fn load_account(name: &str, spendability_address: Option<Address>) -> anyhow::Result<Account> {
    let stored = read_stored_account(name)?;
    let seed = decode_seed(name, &stored.seed)?;

    account_from_seed(&seed, stored.spendability, spendability_address, || {
        prompt_password(name)
    })
}

/// Loads a previously created named account's receiver without requiring its
/// password. Used to look up someone else's receiver (e.g. a transfer
/// recipient), which shouldn't require unlocking their account.
pub fn load_receiver(name: &str) -> anyhow::Result<Receiver> {
    Ok(read_stored_account(name)?.receiver)
}

/// Builds an [`Account`] from a seed and spendability configuration, prompting
/// for a password (via `password`) only if the spendability rule needs one.
fn account_from_seed(
    seed: &[u8; 32],
    spendability: AccountSpendability,
    spendability_address: Option<Address>,
    password: impl FnOnce() -> anyhow::Result<String>,
) -> anyhow::Result<Account> {
    let keys = Keys::from_seed(seed);
    match spendability {
        AccountSpendability::Noop => Ok(Account::from_keys(keys, NoopSpendingAccount)),
        AccountSpendability::Password => {
            let contract_address = spendability_address
                .context("--spendability-address is required for password-spendability accounts")?;
            let secret = password_secret(&password()?);
            let (pk, vk) = load_circuit::<PasswordSpendability>("password")?;
            Ok(Account::from_keys(
                keys,
                PasswordSpendingAccount::new(contract_address, secret, pk, vk),
            ))
        }
    }
}

/// Derives the spendability secret from a password. This is a toy CLI, so
/// this is intentionally simple: no salt, just a direct hash.
fn password_secret(password: &str) -> Fr {
    Fr::from_le_bytes_mod_order(&keccak256(password.as_bytes()).0)
}

/// Prompts for a password, echoing input (this is a toy CLI, not a secure one).
fn prompt_password(name: &str) -> anyhow::Result<String> {
    print!("Password for \"{name}\": ");
    io::stdout().flush()?;

    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    Ok(password.trim_end().to_string())
}

/// Prompts for a new password twice, failing if they don't match.
fn prompt_new_password(name: &str) -> anyhow::Result<String> {
    let password = prompt_password(name)?;

    print!("Confirm password for \"{name}\": ");
    io::stdout().flush()?;
    let mut confirmation = String::new();
    io::stdin().read_line(&mut confirmation)?;

    if password != confirmation.trim_end() {
        anyhow::bail!("passwords do not match");
    }
    Ok(password)
}

/// Reads and parses a previously created named account's file.
fn read_stored_account(name: &str) -> anyhow::Result<StoredAccount> {
    let path = account_file(name);
    let contents = fs::read_to_string(&path).with_context(|| {
        format!(
            "no account named \"{name}\" (looked for {})",
            path.display()
        )
    })?;
    Ok(serde_json::from_str(&contents)?)
}

fn decode_seed(name: &str, seed: &str) -> anyhow::Result<[u8; 32]> {
    let seed_bytes = alloy::primitives::hex::decode(seed)?;
    seed_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("corrupt account file for \"{name}\": seed is not 32 bytes"))
}

/// Loads the cached Groth16 proving/verifying keys from disk, generating and
/// caching them on first use.
pub fn load_circuit<C: ConstraintSynthesizer<Fr> + Default>(
    dir: impl AsRef<Path>,
) -> anyhow::Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>)> {
    let circuit_dir = circuit_dir().join(&dir);

    let pk_path = circuit_dir.join("proving_key.bin.br");
    let vk_path = circuit_dir.join("verifying_key.bin.br");

    if pk_path.exists() && vk_path.exists() {
        info!("Loading cached circuit keys for {}", dir.as_ref().display());
        let pk_bytes =
            fs::read(&pk_path).with_context(|| format!("reading {}", pk_path.display()))?;
        let vk_bytes =
            fs::read(&vk_path).with_context(|| format!("reading {}", vk_path.display()))?;
        let proving_key =
            deserialize_proving_key(&pk_bytes).context("deserializing cached proving key")?;
        let verifying_key =
            deserialize_verifying_key(&vk_bytes).context("deserializing cached verifying key")?;
        return Ok((proving_key, verifying_key));
    }

    info!("Generating circuit keys (first run)...");
    let (pk, vk) = setup_circuit::<C>()?;

    fs::create_dir_all(circuit_dir)?;
    let pk_bytes = serialize_proving_key(&pk).context("serializing proving key")?;
    let vk_bytes = serialize_verifying_key(&vk).context("serializing verifying key")?;
    fs::write(&pk_path, pk_bytes).with_context(|| format!("writing {}", pk_path.display()))?;
    fs::write(&vk_path, vk_bytes).with_context(|| format!("writing {}", vk_path.display()))?;

    Ok((pk, vk))
}

fn base_dir() -> PathBuf {
    PathBuf::from(".tint-cli")
}

fn circuit_dir() -> PathBuf {
    base_dir().join("circuit")
}

fn account_dir(name: &str) -> PathBuf {
    base_dir().join("accounts").join(name)
}

fn account_file(name: &str) -> PathBuf {
    account_dir(name).join("account.json")
}

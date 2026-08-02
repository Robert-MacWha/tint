use std::{
    fs,
    path::{Path, PathBuf},
};

use alloy::primitives::{Address, hex};
use anyhow::Context;
use ark_bn254::{Bn254, Fr};
use ark_groth16::{ProvingKey, VerifyingKey};
use ark_relations::gr1cs::ConstraintSynthesizer;
use serde::{Deserialize, Serialize};
use tint::{
    account::{Account, keys::Keys, receiver::Receiver, spending::NoopSpendingAccount},
    circuit::{
        artifacts::{
            deserialize_matrices, deserialize_proving_key, deserialize_verifying_key,
            serialize_matrices, serialize_proving_key, serialize_verifying_key,
        },
        generate_artifacts,
        matrices::Matrices,
    },
};
use tint_multisig_spendability::N_SIGNERS;
use tracing::info;

use crate::account::{
    multisig::{create_multisig_account, load_multisig_account},
    password::{create_password_account, load_password_account},
};

#[derive(Serialize, Deserialize)]
struct StoredAccount {
    seed: String,
    receiver: Receiver,
    spendability: SpendabilityState,
}

/// Spendability-specific persistent state.
#[derive(Serialize, Deserialize)]
pub enum SpendabilityState {
    Noop,
    Password,
    Multisig { signers: [String; N_SIGNERS] },
}

#[derive(Clone, clap::ValueEnum)]
pub enum AccountSpendability {
    Noop,
    Password,
    Multisig,
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
    let keys = Keys::from_seed(&seed);

    let (account, state) = match spendability {
        AccountSpendability::Noop => (
            Account::from_keys(keys, NoopSpendingAccount),
            SpendabilityState::Noop,
        ),
        AccountSpendability::Password => create_password_account(keys, spendability_address)?,
        AccountSpendability::Multisig => create_multisig_account(name, keys, spendability_address)?,
    };

    let stored = StoredAccount {
        seed: hex::encode_prefixed(seed),
        receiver: account.receiver(),
        spendability: state,
    };
    fs::write(account_file(name), serde_json::to_string_pretty(&stored)?)
        .with_context(|| format!("writing account file for \"{name}\""))?;

    Ok(account)
}

/// Loads a previously created named account
pub fn load_account(name: &str, spendability_address: Option<Address>) -> anyhow::Result<Account> {
    let stored = read_stored_account(name)?;
    let seed = decode_seed(name, &stored.seed)?;
    let keys = Keys::from_seed(&seed);

    match stored.spendability {
        SpendabilityState::Noop => Ok(Account::from_keys(keys, NoopSpendingAccount)),
        SpendabilityState::Password => load_password_account(keys, spendability_address),
        SpendabilityState::Multisig { signers } => {
            load_multisig_account(keys, spendability_address, &signers)
        }
    }
}

/// Loads a previously created named account's receiver.
pub fn load_receiver(name: &str) -> anyhow::Result<Receiver> {
    Ok(read_stored_account(name)?.receiver)
}

fn decode_seed(name: &str, seed: &str) -> anyhow::Result<[u8; 32]> {
    let seed_bytes = hex::decode(seed)?;
    seed_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("corrupt account file for \"{name}\": seed is not 32 bytes"))
}

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

/// Loads the cached Groth16 proving/verifying keys from disk, generating and
/// caching them on first use.
pub fn load_circuit<C: ConstraintSynthesizer<Fr> + Default>(
    dir: impl AsRef<Path>,
) -> anyhow::Result<(Matrices<Fr>, ProvingKey<Bn254>, VerifyingKey<Bn254>)> {
    let circuit_dir = circuit_dir().join(&dir);

    let matrices_path = circuit_dir.join("matrices.bin.br");
    let pk_path = circuit_dir.join("proving_key.bin.br");
    let vk_path = circuit_dir.join("verifying_key.bin.br");

    if matrices_path.exists() && pk_path.exists() && vk_path.exists() {
        info!("Loading cached circuit keys for {}", dir.as_ref().display());
        let matrices_bytes = fs::read(&matrices_path).context("reading cached matrices")?;
        let pk_bytes = fs::read(&pk_path).context("reading cached proving key")?;
        let vk_bytes = fs::read(&vk_path).context("reading cached verifying key")?;
        let matrices =
            deserialize_matrices(&matrices_bytes).context("deserializing cached matrices")?;
        let proving_key =
            deserialize_proving_key(&pk_bytes).context("deserializing cached proving key")?;
        let verifying_key =
            deserialize_verifying_key(&vk_bytes).context("deserializing cached verifying key")?;
        return Ok((matrices, proving_key, verifying_key));
    }

    info!("Generating circuit keys (first run)...");
    let (matrices, pk, vk) = generate_artifacts::<C>()?;

    fs::create_dir_all(circuit_dir)?;
    let pk_bytes = serialize_proving_key(&pk).context("serializing proving key")?;
    let vk_bytes = serialize_verifying_key(&vk).context("serializing verifying key")?;
    let matrices_bytes = serialize_matrices(&matrices).context("serializing matrices")?;
    fs::write(&pk_path, pk_bytes).context("writing proving key")?;
    fs::write(&vk_path, vk_bytes).context("writing verifying key")?;
    fs::write(&matrices_path, matrices_bytes).context("writing matrices")?;

    Ok((matrices, pk, vk))
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

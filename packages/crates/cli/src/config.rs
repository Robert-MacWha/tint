use std::{
    fs,
    path::{Path, PathBuf},
};

use alloy::primitives::{Address, B256};
use anyhow::Context;
use ark_bn254::{Bn254, Fr};
use ark_relations::gr1cs::ConstraintSynthesizer;
use serde::{Deserialize, Serialize};
use tint::{
    account::Account,
    fr::{b256_to_fr, fr_to_b256},
    policy::Policy,
    syncer::Snapshot,
};
use tint_groth16::{
    artifacts::Artifacts,
    serde::{
        deserialize_matrices, deserialize_pk, deserialize_vk, serialize_matrices, serialize_pk,
        serialize_vk,
    },
};
use tint_intent_policy::intent::{CancelKey, Intent};
use tint_multisig_policy::N_SIGNERS;
use tracing::info;

/// A local account: its seed, its notes' policy, and how to authorize spends.
#[derive(Serialize, Deserialize)]
pub struct StoredAccount {
    pub seed: B256,
    pub policy_address: Address,
    pub policy_witness: B256,
    pub spend: SpendPolicy,
}

/// An intent this wallet posted, with the secrets needed to claim or cancel it.
#[derive(Serialize, Deserialize)]
pub struct StoredIntent {
    pub intent: Intent,
    pub cancel_key: B256,
    /// Seed of the one-time account that receives the intent's wanted note.
    pub receiver_seed: B256,
}

/// Policy-specific state needed to authorize spends.
#[derive(Serialize, Deserialize)]
pub enum SpendPolicy {
    None,
    Password,
    Multisig { signers: [String; N_SIGNERS] },
    HardwareWallet(HardwareSigner),
    Timelock { unlock_time: u64, salt: B256 },
}

/// The signer behind a hardware-wallet account.
#[derive(Serialize, Deserialize)]
pub enum HardwareSigner {
    /// A Ledger's Ledger Live account at this index.
    Ledger { index: usize },
    /// A local private key, signing through the same path as a device.
    PrivateKey(B256),
}

impl StoredAccount {
    pub fn new(seed: B256, policy: Policy, spend: SpendPolicy) -> Self {
        Self {
            seed,
            policy_address: policy.address,
            policy_witness: fr_to_b256(policy.witness),
            spend,
        }
    }

    pub fn policy(&self) -> Policy {
        Policy::new(self.policy_address, b256_to_fr(self.policy_witness))
    }

    pub fn account(&self) -> Account {
        Account::from_seed(&self.seed.0, self.policy())
    }
}

impl StoredIntent {
    pub fn cancel_key(&self) -> CancelKey {
        CancelKey(b256_to_fr(self.cancel_key))
    }

    pub fn receiver(&self) -> Account {
        Account::from_seed(&self.receiver_seed.0, Policy::NONE)
    }
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
    accounts.sort();
    Ok(accounts)
}

/// Saves a new named account, failing if one with this name already exists.
pub fn save_new_account(name: &str, account: &StoredAccount) -> anyhow::Result<()> {
    let dir = account_dir(name);
    if dir.exists() {
        anyhow::bail!("account \"{name}\" already exists at {}", dir.display());
    }
    fs::create_dir_all(&dir)?;
    fs::write(account_file(name), serde_json::to_string_pretty(account)?)
        .with_context(|| format!("writing account file for \"{name}\""))
}

pub fn load_account(name: &str) -> anyhow::Result<StoredAccount> {
    let path = account_file(name);
    let contents = fs::read_to_string(&path).with_context(|| {
        format!(
            "no account named \"{name}\" (looked for {})",
            path.display()
        )
    })?;
    serde_json::from_str(&contents).with_context(|| format!("corrupt account file for \"{name}\""))
}

/// Loads a snapshot of the Tint deployment at `tint` on `chain_id`, if one was saved.
pub fn load_snapshot(chain_id: u64, tint: Address) -> anyhow::Result<Option<Snapshot>> {
    let path = snapshot_file(chain_id, tint);
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read(&path).context("reading snapshot")?;
    Ok(Some(
        serde_json::from_slice(&contents).context("corrupt snapshot")?,
    ))
}

pub fn save_snapshot(chain_id: u64, snapshot: &Snapshot) -> anyhow::Result<()> {
    let path = snapshot_file(chain_id, snapshot.tint);
    fs::create_dir_all(base_dir().join("snapshots"))?;
    fs::write(path, serde_json::to_vec(snapshot)?).context("writing snapshot")
}

/// Loads the intents `name` posted, returning an empty list if none exist.
pub fn load_intents(name: &str) -> anyhow::Result<Vec<StoredIntent>> {
    let path = account_dir(name).join("intents.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&path).context("reading intents")?;
    serde_json::from_str(&contents).context("corrupt intents file")
}

pub fn save_intents(name: &str, intents: &[StoredIntent]) -> anyhow::Result<()> {
    let path = account_dir(name).join("intents.json");
    fs::write(path, serde_json::to_string_pretty(intents)?).context("writing intents")
}

/// Loads the cached Groth16 proving/verifying keys from disk, generating and
/// caching them on first use.
pub fn load_circuit<C: ConstraintSynthesizer<Fr> + Default>(
    dir: impl AsRef<Path>,
) -> anyhow::Result<Artifacts<Bn254>> {
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
        let pk = deserialize_pk(&pk_bytes).context("deserializing cached proving key")?;
        let vk = deserialize_vk(&vk_bytes).context("deserializing cached verifying key")?;
        return Ok(Artifacts { matrices, pk, vk });
    }

    info!("Generating circuit keys (first run)...");
    let artifacts = Artifacts::generate_deterministic::<C>()?;

    fs::create_dir_all(circuit_dir)?;
    let pk_bytes = serialize_pk(&artifacts.pk).context("serializing proving key")?;
    let vk_bytes = serialize_vk(&artifacts.vk).context("serializing verifying key")?;
    let matrices_bytes = serialize_matrices(&artifacts.matrices).context("serializing matrices")?;
    fs::write(&pk_path, pk_bytes).context("writing proving key")?;
    fs::write(&vk_path, vk_bytes).context("writing verifying key")?;
    fs::write(&matrices_path, matrices_bytes).context("writing matrices")?;

    Ok(artifacts)
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

fn snapshot_file(chain_id: u64, tint: Address) -> PathBuf {
    base_dir()
        .join("snapshots")
        .join(format!("{chain_id}-{tint}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_account_round_trips_its_account() {
        let policy = Policy::new(Address::repeat_byte(1), Fr::from(42u64));
        let stored = StoredAccount::new(
            B256::repeat_byte(7),
            policy,
            SpendPolicy::Timelock {
                unlock_time: 1_000,
                salt: B256::repeat_byte(2),
            },
        );

        let json = serde_json::to_string(&stored).unwrap();
        let decoded: StoredAccount = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.policy(), policy);
        assert_eq!(
            decoded.account().pub_account(),
            stored.account().pub_account()
        );
    }
}

use std::io::{self, Write};

use alloy::primitives::{Address, keccak256};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use tint::account::{Account, keys::Keys};
use tint_password_spendability::{account::PasswordSpendingAccount, circuit::PasswordSpendability};

use crate::{
    account::resolve_spendability_address,
    config::{SpendabilityState, load_circuit},
};

const SPENDABILITY_ADDRESS_ENV_VAR: &str = "PASSWORD_SPENDABILITY_ADDRESS";

pub fn create_password_account(
    name: &str,
    keys: Keys,
) -> anyhow::Result<(Account, SpendabilityState)> {
    let address = resolve_spendability_address(SPENDABILITY_ADDRESS_ENV_VAR, name)?;
    let account = password_account(keys, address)?;
    Ok((account, SpendabilityState::Password { address }))
}

pub fn load_password_account(keys: Keys, address: Address) -> anyhow::Result<Account> {
    password_account(keys, address)
}

fn password_account(keys: Keys, contract_address: Address) -> anyhow::Result<Account> {
    let artifacts = load_circuit::<PasswordSpendability>("password")?;

    let account = PasswordSpendingAccount::new(contract_address, prompt_password, artifacts)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(Account::from_keys(keys, account))
}

fn prompt_password() -> Result<Fr, Box<dyn std::error::Error + Send + Sync>> {
    print!("Password: ");
    io::stdout().flush()?;

    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim_end().to_string();
    let secret = Fr::from_le_bytes_mod_order(&keccak256(password.as_bytes()).0);
    Ok(secret)
}

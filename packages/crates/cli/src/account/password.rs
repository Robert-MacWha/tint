use std::io::{self, Write};

use alloy::primitives::{Address, keccak256};
use anyhow::Context;
use ark_bn254::Fr;
use ark_ff::PrimeField;
use tint::account::{Account, keys::Keys};
use tint_password_spendability::{account::PasswordSpendingAccount, circuit::PasswordSpendability};

use crate::config::{SpendabilityState, load_circuit};

pub fn create_password_account(
    keys: Keys,
    spendability_address: Option<Address>,
) -> anyhow::Result<(Account, SpendabilityState)> {
    let account = password_account(keys, spendability_address)?;
    Ok((account, SpendabilityState::Password))
}

pub fn load_password_account(
    keys: Keys,
    spendability_address: Option<Address>,
) -> anyhow::Result<Account> {
    let account = password_account(keys, spendability_address)?;
    Ok(account)
}

fn password_account(keys: Keys, spendability_address: Option<Address>) -> anyhow::Result<Account> {
    let contract_address = spendability_address
        .context("--spendability-address is required for password-spendability accounts")?;

    let (matrices, pk, vk) = load_circuit::<PasswordSpendability>("password")?;

    let account = PasswordSpendingAccount::new(contract_address, prompt_password, matrices, pk, vk)
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

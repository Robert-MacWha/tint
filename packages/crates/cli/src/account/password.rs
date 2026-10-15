use std::io::{self, Write};

use alloy::primitives::{Address, keccak256};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use tint_password_spendability::{circuit::PasswordSpendability, policy::PasswordPolicy};

use crate::{
    account::{Authorizer, resolve_spendability_address},
    config::{SpendabilityState, load_circuit},
};

const SPENDABILITY_ADDRESS_ENV_VAR: &str = "PASSWORD_SPENDABILITY_ADDRESS";

pub fn create_password_account(name: &str) -> anyhow::Result<(Authorizer, SpendabilityState)> {
    let address = resolve_spendability_address(SPENDABILITY_ADDRESS_ENV_VAR, name)?;
    let authorizer = load_password_authorizer(address)?;
    Ok((authorizer, SpendabilityState::Password { address }))
}

pub fn load_password_authorizer(contract_address: Address) -> anyhow::Result<Authorizer> {
    let artifacts = load_circuit::<PasswordSpendability>("password")?;

    let password = prompt_password()?;
    let policy = PasswordPolicy::new(contract_address, password, artifacts);
    Ok(Authorizer::Password(Box::new(policy)))
}

fn prompt_password() -> io::Result<Fr> {
    print!("Password: ");
    io::stdout().flush()?;

    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim_end().to_string();
    let secret = Fr::from_le_bytes_mod_order(&keccak256(password.as_bytes()).0);
    Ok(secret)
}

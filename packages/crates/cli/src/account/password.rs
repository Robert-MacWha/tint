use std::io::{self, Write};

use alloy::primitives::{Address, keccak256};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use tint_password_policy::{circuit::PasswordPolicyCircuit, policy::PasswordPolicy};

use crate::{
    account::{Authorizer, resolve_policy_address},
    config::{PolicyState, load_circuit},
};

const POLICY_ADDRESS_ENV_VAR: &str = "PASSWORD_POLICY_ADDRESS";

pub fn create_password_account(name: &str) -> anyhow::Result<(Authorizer, PolicyState)> {
    let address = resolve_policy_address(POLICY_ADDRESS_ENV_VAR, name)?;
    let authorizer = load_password_authorizer(address)?;
    Ok((authorizer, PolicyState::Password { address }))
}

pub fn load_password_authorizer(contract_address: Address) -> anyhow::Result<Authorizer> {
    let artifacts = load_circuit::<PasswordPolicyCircuit>("password")?;

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

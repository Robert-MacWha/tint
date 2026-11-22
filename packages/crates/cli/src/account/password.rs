use std::io::{self, Write};

use alloy::primitives::{Address, keccak256};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use tint::policy::Policy;
use tint_password_policy::{circuit::PasswordPolicyCircuit, policy::PasswordPolicy};

use crate::{account::Authorizer, config::load_circuit};

/// Prompts for a new account's password, returning its policy.
pub fn create(contract_address: Address) -> anyhow::Result<Policy> {
    Ok(password_policy(contract_address)?.policy())
}

pub fn load(contract_address: Address) -> anyhow::Result<Authorizer> {
    Ok(Authorizer::Password(Box::new(password_policy(
        contract_address,
    )?)))
}

fn password_policy(contract_address: Address) -> anyhow::Result<PasswordPolicy> {
    let artifacts = load_circuit::<PasswordPolicyCircuit>("password")?;
    let password = prompt_password()?;
    Ok(PasswordPolicy::new(contract_address, password, artifacts))
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

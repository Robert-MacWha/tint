use alloy::primitives::{Address, B256};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use tint::{fr::fr_to_b256, policy::Policy};
use tint_timelock_policy::{circuit::TimelockPolicyCircuit, policy::TimelockPolicy};

use crate::{
    account::Authorizer,
    config::{SpendPolicy, load_circuit},
};

/// Picks a random salt for a new account unlocking at `unlock_time`.
pub fn create(address: Address, unlock_time: u64) -> anyhow::Result<(Policy, SpendPolicy)> {
    let mut bytes = [0u8; 32];
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut bytes);
    let salt = fr_to_b256(Fr::from_le_bytes_mod_order(&bytes));

    let policy = timelock_policy(address, unlock_time, salt)?.policy();
    Ok((policy, SpendPolicy::Timelock { unlock_time, salt }))
}

pub fn load(address: Address, unlock_time: u64, salt: B256) -> anyhow::Result<Authorizer> {
    Ok(Authorizer::Timelock(Box::new(timelock_policy(
        address,
        unlock_time,
        salt,
    )?)))
}

fn timelock_policy(
    address: Address,
    unlock_time: u64,
    salt: B256,
) -> anyhow::Result<TimelockPolicy> {
    let artifacts = load_circuit::<TimelockPolicyCircuit>("timelock")?;
    Ok(TimelockPolicy::new(
        address,
        unlock_time,
        tint::fr::b256_to_fr(salt),
        artifacts,
    ))
}

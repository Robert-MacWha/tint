mod common;

use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    account::Account, circuit::join_split::JoinSplitCircuit, operation::OperationBuilder,
    policy::Policy,
};
use tint_groth16::artifacts::Artifacts;
use tint_password_policy::{circuit::PasswordPolicyCircuit, policy::PasswordPolicy};
use tracing::info;

use crate::common::{
    anvil::{Instance, send},
    policy::deploy_policy,
    util::{assert_unspent, init_tracing},
};

/// Test that we can spend a note using the `PasswordPolicyCircuit`.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn password() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;
    let policy = deploy_policy(&instance.provider).await?;

    let tint = *instance.tint.address();
    let asset = instance.asset();

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;
    let policy_artifacts = Artifacts::generate_deterministic::<PasswordPolicyCircuit>()?;

    // Setup policy
    info!("Setting up password policy...");
    let password = PasswordPolicy::new(policy, Fr::from(1234), policy_artifacts);
    let account_1 = Account::from_seed(&[11u8; 32], password.policy());
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    let amount = 1_000u128;
    instance
        .deposit(account_1.pub_account(), amount, &mut rng)
        .await?;

    let (snapshot, mmr) = instance.sync().await?;
    let notes = snapshot.unspent(&account_1);

    // Spend the note, which is only spendable using the `PasswordPolicy` contract / circuit.
    info!("Spending note");
    let mut op = OperationBuilder::new()
        .spend(notes[0].clone())
        .output(account_1.pub_account(), asset, amount - 150)
        .output(account_2.pub_account(), asset, 100)
        .withdraw(Address::new([3; 20]), asset, 50)
        .build(&mut rng)?;
    password.authorize(&mut op)?;
    let proven = op.prove_with_mmr(&mmr, &artifacts, &mut rng)?;
    send(&instance.provider, proven.transaction(tint)).await?;

    // Verify balances
    info!("Verifying balances");
    let (snapshot, _) = instance.sync().await?;
    assert_unspent(&snapshot, &account_1, &[(asset, amount - 150)]);
    assert_unspent(&snapshot, &account_2, &[(asset, 100)]);

    Ok(())
}

mod common;

use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    deposit::Deposit,
    note::asset::AssetId,
    operation::OperationBuilder,
    policy::Policy,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Syncer, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;
use tint_password_policy::{circuit::PasswordPolicyCircuit, policy::PasswordPolicy};
use tracing::info;

use crate::common::{
    anvil::{self, send},
    policy::deploy_policy,
};

/// Test that we can spend a note using the `PasswordPolicyCircuit`.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn password() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse()?)
        .add_directive("r1cs=off".parse()?);

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = anvil::setup().await?;
    let policy = deploy_policy(&instance.provider).await?;

    let provider = instance.provider;
    let tint = *instance.tint.address();
    let token = instance.token;

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;
    let policy_artifacts = Artifacts::generate_deterministic::<PasswordPolicyCircuit>()?;

    // Setup policy
    info!("Setting up password policy...");
    let password = PasswordPolicy::new(policy, Fr::from(1234), policy_artifacts);
    let account_1 = Account::from_seed(&[11u8; 32], password.policy());
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    // Deposit into Tint.
    info!("Depositing into Tint");
    let asset = AssetId::from(*token.address());
    let amount = 1_000u128;

    let deposit = Deposit::new(tint, account_1.pub_account(), asset, amount, &mut rng);
    send(&provider, deposit.approval()).await?;
    send(&provider, deposit.transaction(&mut rng)?).await?;

    info!("Syncing");
    let syncer = RpcSyncer::new(provider.clone());
    let snapshot = syncer.sync(tint, ..).await?;
    let mut mmr = SkewMmr::new();
    mmr.append_events(&snapshot.events)?;

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
    send(&provider, proven.transaction(tint)).await?;

    info!("Syncing");
    let snapshot = syncer.sync(tint, ..).await?;

    // Verify balances
    info!("Verifying balances");
    let notes_1 = snapshot.unspent(&account_1);
    assert_eq!(notes_1.len(), 1);
    assert_eq!(notes_1[0].note.amount, amount - 150);
    assert_eq!(notes_1[0].note.asset, asset);

    let notes_2 = snapshot.unspent(&account_2);
    assert_eq!(notes_2.len(), 1);
    assert_eq!(notes_2[0].note.amount, 100);
    assert_eq!(notes_2[0].note.asset, asset);

    Ok(())
}

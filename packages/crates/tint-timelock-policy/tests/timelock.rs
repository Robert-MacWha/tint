mod common;

use alloy_primitives::{Address, U64};
use alloy_provider::Provider;
use alloy_rpc_types_eth::BlockNumberOrTag;
use alloy_sol_types::SolError;
use ark_bn254::Fr;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    account::Account, circuit::join_split::JoinSplitCircuit, operation::OperationBuilder,
    policy::Policy,
};
use tint_groth16::artifacts::Artifacts;
use tint_timelock_policy::{circuit::TimelockPolicyCircuit, policy::TimelockPolicy};
use tracing::info;

use crate::common::{
    anvil::{Instance, send},
    policy::{TimelockPolicy::Locked, deploy_policy},
    util::{assert_unspent, init_tracing},
};

const HOUR: u64 = 3600;

/// Test that a note under the `TimelockPolicyCircuit` is only spendable once unlocked.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn timelock() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;
    let policy = deploy_policy(&instance.provider).await?;

    let tint = *instance.tint.address();
    let asset = instance.asset();

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;
    let policy_artifacts = Artifacts::generate_deterministic::<TimelockPolicyCircuit>()?;

    // Setup policy, locking notes for an hour
    info!("Setting up timelock policy...");
    let unlock_time = latest_timestamp(&instance.provider).await? + HOUR;
    let timelock = TimelockPolicy::new(policy, unlock_time, Fr::from(1234), policy_artifacts);
    let account_1 = Account::from_seed(&[11u8; 32], timelock.policy());
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    let amount = 1_000u128;
    instance
        .deposit(account_1.pub_account(), amount, &mut rng)
        .await?;

    let (snapshot, mmr) = instance.sync().await?;
    let notes = snapshot.unspent(&account_1);

    // Prove the spend against a bound rounded up to the hour, hiding the exact unlock time.
    info!("Spending note");
    let bound = unlock_time.next_multiple_of(HOUR);
    let mut op = OperationBuilder::new(&account_1.encryption_key)
        .spend(notes[0].clone())
        .output(account_1.pub_account(), asset, amount - 150)
        .output(account_2.pub_account(), asset, 100)
        .withdraw(Address::new([3; 20]), asset, 50)
        .build(&mut rng)?;
    timelock.authorize(&mut op, bound)?;
    let proven = op.prove_with_mmr(&mmr, &artifacts, &mut rng)?;

    info!("Spending before the bound fails");
    // `call` rather than `send`, so the failed attempt doesn't consume a nonce.
    let locked = instance
        .provider
        .call(proven.transaction(tint))
        .await
        .expect_err("spent a note before its unlock time");
    let revert = locked
        .as_error_resp()
        .expect("expected an error response")
        .as_revert_data();
    assert_eq!(
        revert.as_deref().map(|r| &r[..]),
        Some(&Locked::SELECTOR[..])
    );

    info!("Spending after the bound succeeds");
    let _: i64 = instance
        .provider
        .raw_request("evm_increaseTime".into(), (U64::from(2 * HOUR),))
        .await?;
    let _: String = instance
        .provider
        .raw_request("evm_mine".into(), (None::<()>,))
        .await?;
    send(&instance.provider, proven.transaction(tint)).await?;

    // Verify balances
    info!("Verifying balances");
    let (snapshot, _) = instance.sync().await?;
    assert_unspent(&snapshot, &account_1, &[(asset, amount - 150)]);
    assert_unspent(&snapshot, &account_2, &[(asset, 100)]);

    Ok(())
}

/// The latest block's timestamp.
async fn latest_timestamp(provider: &impl Provider) -> anyhow::Result<u64> {
    let block = provider
        .get_block_by_number(BlockNumberOrTag::Latest)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no latest block"))?;
    Ok(block.header.timestamp)
}

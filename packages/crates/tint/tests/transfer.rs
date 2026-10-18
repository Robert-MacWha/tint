use alloy_primitives::U256;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    operation::OperationBuilder,
    policy::Policy,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Syncer, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;
use tracing::info;

use crate::common::{
    anvil::{Instance, send},
    util::{assert_unspent, init_tracing},
};

mod common;

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn transfer() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;

    let provider = instance.provider.clone();
    let tint = *instance.tint.address();
    let asset = instance.asset();

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    let account_1 = Account::from_seed(&[11u8; 32], Policy::NONE);
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    let amount = 1_000u128;
    instance
        .deposit(account_1.pub_account(), amount, &mut rng)
        .await?;

    info!("Syncing");
    let syncer = RpcSyncer::new(provider.clone());
    let snapshot = syncer.sync(tint, ..).await?;
    let mut mmr = SkewMmr::new();
    mmr.append_events(&snapshot.events)?;

    // Transfer to another account
    info!("Transferring to another account");
    let notes = snapshot.unspent(&account_1);

    let proven = OperationBuilder::new()
        .spend(notes[0].clone())
        .output(account_1.pub_account(), asset, 100)
        .output(account_2.pub_account(), asset, amount - 100)
        .build(&mut rng)?
        .prove_with_mmr(&mmr, &artifacts, &mut rng)?;
    let transfer_receipt = send(&provider, proven.transaction(tint)).await?;
    info!("Transferred for {} gas", transfer_receipt.gas_used);

    info!("Syncing");
    let snapshot = syncer.sync(tint, ..).await?;

    // Verify balances
    info!("Verifying balances");
    assert_unspent(&snapshot, &account_1, &[(asset, 100)]);
    assert_unspent(&snapshot, &account_2, &[(asset, amount - 100)]);
    assert_eq!(
        instance.token.balanceOf(tint).call().await?,
        U256::from(amount)
    );

    Ok(())
}

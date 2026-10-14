use alloy_primitives::{Bytes, U256};
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
use tracing::info;

use crate::common::anvil::{self, send};

mod common;

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn transfer() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse()?)
        .add_directive("r1cs=off".parse()?);

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = anvil::setup().await?;

    let provider = instance.provider;
    let tint = *instance.tint.address();
    let token = instance.token;

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    let account_1 = Account::from_seed(&[11u8; 32], Policy::NONE);
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    // Deposit into Tint
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

    // Transfer to another account
    info!("Transferring to another account");
    let notes = snapshot.unspent(&account_1);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].note.amount, amount);
    assert_eq!(notes[0].note.asset, asset);

    let proven = OperationBuilder::new()
        .spend(notes[0].clone())
        .output(account_1.pub_account(), asset, 100)
        .output(account_2.pub_account(), asset, amount - 100)
        .build(&mut rng)?
        .authorize(vec![Bytes::default()])?
        .prove_with_mmr(&mmr, &artifacts, &mut rng)?;
    let transfer_receipt = send(&provider, proven.transaction(tint)).await?;
    info!("Transferred for {} gas", transfer_receipt.gas_used);

    info!("Syncing");
    let snapshot = syncer.sync(tint, ..).await?;

    // Verify balances
    info!("Verifying balances");
    let notes_1 = snapshot.unspent(&account_1);
    assert_eq!(notes_1.len(), 1);
    assert_eq!(notes_1[0].note.amount, 100);
    assert_eq!(notes_1[0].note.asset, asset);

    let notes_2 = snapshot.unspent(&account_2);
    assert_eq!(notes_2.len(), 1);
    assert_eq!(notes_2[0].note.amount, amount - 100);
    assert_eq!(notes_2[0].note.asset, asset);

    assert_eq!(token.balanceOf(tint).call().await?, U256::from(amount));

    Ok(())
}

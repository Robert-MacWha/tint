use alloy_primitives::U256;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    account::Account,
    deposit::Deposit,
    note::asset::AssetId,
    policy::Policy,
    syncer::{Syncer, rpc::RpcSyncer},
};
use tracing::info;

use crate::common::anvil::{self, send};

mod common;

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn shield() -> Result<(), Box<dyn std::error::Error>> {
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

    let account = Account::from_seed(&[11u8; 32], Policy::NONE);

    // Deposit into Tint.
    info!("Depositing into Tint");
    let asset = AssetId::from(*token.address());
    let amount = 1_000u128;

    let deposit = Deposit::new(tint, account.pub_account(), asset, amount, &mut rng);
    send(&provider, deposit.approval()).await?;
    let shield_receipt = send(&provider, deposit.transaction(&mut rng)?).await?;
    info!("Shielded for {} gas", shield_receipt.gas_used);

    info!("Syncing");
    let snapshot = RpcSyncer::new(provider.clone()).sync(tint, ..).await?;

    // Verify balances
    info!("Verifying balances");
    assert_eq!(token.balanceOf(tint).call().await?, U256::from(amount));

    let notes = snapshot.unspent(&account);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].note, deposit.note());

    Ok(())
}

use std::sync::Arc;

use alloy_primitives::{Address, U256};
use ark_std::rand::{Rng, rngs::StdRng};
use rand_core::SeedableRng;
use tint::{
    account::{Account, keys::Keys, spending::NoopSpendingAccount},
    circuit::join_split::JoinSplitCircuit,
    database::memory::MemoryDatabase,
    indexer::{Indexer, syncer::RpcSyncer, verifier::RpcVerifier},
    note::asset::AssetId,
    provider::Provider,
};
use tint_groth16::artifacts::Artifacts;
use tracing::info;

use crate::common::anvil;

mod common;

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn unshield() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse().unwrap())
        .add_directive("r1cs=off".parse().unwrap());

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = anvil::setup().await.unwrap();

    let provider = instance.provider;
    let tint = instance.tint;
    let token = instance.token;

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>().unwrap();

    // Setup tint provider
    info!("Setting up tint provider...");
    let account_1 = Account::from_keys(Keys::from_seed(&[11u8; 32]), NoopSpendingAccount);
    let unshield_address = Address::new(rng.r#gen());

    let syncer = Arc::new(RpcSyncer::new(provider.clone(), *tint.address()));
    let verifier = Arc::new(RpcVerifier::new(provider.clone(), *tint.address()));
    let database = Arc::new(MemoryDatabase::default());
    let indexer = Indexer::new(syncer, verifier, database).await.unwrap();
    let mut tint_provider = Provider::new(indexer, artifacts);
    tint_provider.add_account(account_1.clone()).await.unwrap();

    // Approve Tint to pull the deposit.
    let _ = token
        .approve(*tint.address(), U256::MAX)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Deposit into Tint
    info!("Depositing into Tint");
    let asset = AssetId::from(*token.address());
    let amount = 1_000u128;

    let call = tint_provider
        .deposit(account_1.receiver(), asset, amount, &mut rng)
        .unwrap();
    tint.call_builder(&call)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    info!("Syncing");
    tint_provider.sync().await.unwrap();

    // Unshield
    info!("Unshielding");
    let notes = tint_provider.notes(account_1.receiver());

    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].inner.amount, amount);
    assert_eq!(notes[0].inner.asset, asset);

    let call = tint_provider
        .operate(
            [notes[0].clone()],
            [(account_1.receiver(), asset, 100)],
            [(unshield_address, asset, amount - 100)],
            &mut rng,
        )
        .await
        .unwrap();

    let unshield_receipt = tint
        .call_builder(&call)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    info!("Unshielded for {} gas", unshield_receipt.gas_used);
    info!("Syncing");
    tint_provider.sync().await.unwrap();

    // Verify balances
    info!("Verifying balances");

    let notes_1 = tint_provider.notes(account_1.receiver());
    assert_eq!(notes_1.len(), 1);
    assert_eq!(notes_1[0].inner.amount, 100);
    assert_eq!(notes_1[0].inner.asset, asset);

    assert_eq!(
        token.balanceOf(unshield_address).call().await.unwrap(),
        U256::from(amount - 100)
    );
}

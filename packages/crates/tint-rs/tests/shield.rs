use std::sync::Arc;

use alloy_primitives::{Address, B256, U256};
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint_rs::{
    account::{keys::Keys, receiver::Receiver},
    circuit::setup_circuits,
    database::memory::MemoryDatabase,
    indexer::{Indexer, syncer::RpcSyncer, verifier::RpcVerifier},
    note::asset::AssetId,
    provider::Provider,
};
use tracing::info;

use crate::common::anvil;

mod common;

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn shield() {
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
    println!("Setting up circuits...");
    let (proving_key, verifying_key) = setup_circuits().unwrap();

    // Setup tint provider
    println!("Setting up tint provider...");
    let keys = Keys::from_seed(&[11u8; 32]);

    let syncer = Arc::new(RpcSyncer::new(provider.clone(), *tint.address()));
    let verifier = Arc::new(RpcVerifier::new(provider.clone(), *tint.address()));
    let database = Arc::new(MemoryDatabase::default());
    let indexer = Indexer::new(
        syncer,
        verifier,
        database,
        keys.nullifier_key.clone(),
        keys.encryption_key.clone(),
    );
    let mut tint_provider = Provider::new(indexer, proving_key, verifying_key);

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
    let receiver = Receiver::new(
        keys.nullifier_pub_key(),
        keys.encryption_pub_key(),
        Address::ZERO,
        B256::ZERO,
    );
    let asset = AssetId::from(*token.address());
    let amount = 1_000u128;

    // Warmup
    let call = tint_provider
        .deposit(&receiver, asset, amount, &mut rng)
        .unwrap();
    let receipt_1 = tint
        .call_builder(&call)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Deposit
    let call = tint_provider
        .deposit(&receiver, asset, amount, &mut rng)
        .unwrap();
    let receipt_2 = tint
        .call_builder(&call)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Verify balances
    info!("Syncing");
    tint_provider.sync().await.unwrap();

    info!("Verifying balances");

    assert_eq!(
        token.balanceOf(*tint.address()).call().await.unwrap(),
        U256::from(amount * 2)
    );

    let notes = tint_provider.spendable_notes();
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].base.amount, amount);
    assert_eq!(notes[0].base.asset, asset);
    assert_eq!(notes[1].base.amount, amount);
    assert_eq!(notes[1].base.asset, asset);

    // Output gas benchmark
    info!("Gas used for deposit_1: {}", receipt_1.gas_used);
    info!("Gas used for deposit_2: {}", receipt_2.gas_used);
}

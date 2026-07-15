use std::sync::Arc;

use alloy_primitives::{Address, B256, U256};
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint_rs::{
    account::{Account, keys::Keys},
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
async fn transfer() {
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
    let (proving_key, verifying_key) = setup_circuits().unwrap();

    // Setup tint provider
    info!("Setting up tint provider...");
    let account_1 = Account::new(Keys::from_seed(&[11u8; 32]), Address::ZERO, B256::ZERO);
    let account_2 = Account::new(Keys::from_seed(&[22u8; 32]), Address::ZERO, B256::ZERO);

    let syncer = Arc::new(RpcSyncer::new(provider.clone(), *tint.address()));
    let verifier = Arc::new(RpcVerifier::new(provider.clone(), *tint.address()));
    let database = Arc::new(MemoryDatabase::default());
    let indexer = Indexer::new(syncer, verifier, database);
    let mut tint_provider = Provider::new(indexer, proving_key, verifying_key);
    tint_provider.add_account(account_1.clone());
    tint_provider.add_account(account_2.clone());

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

    // Transfer to another account
    info!("Transferring to another account");
    let notes = tint_provider.spendable_notes(account_1.receiver());

    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].base.amount, amount);
    assert_eq!(notes[0].base.asset, asset);

    let call = tint_provider
        .operate(
            [notes[0].clone()],
            [
                (account_1.receiver(), asset, 100),
                (account_2.receiver(), asset, amount - 100),
            ],
            [],
            &mut rng,
        )
        .unwrap();

    let transfer_receipt = tint
        .call_builder(&call)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    info!("Transferred for {} gas", transfer_receipt.gas_used);
    info!("Syncing");
    tint_provider.sync().await.unwrap();

    // Verify balances
    info!("Verifying balances");

    let notes_1 = tint_provider.spendable_notes(account_1.receiver());
    assert_eq!(notes_1.len(), 1);
    assert_eq!(notes_1[0].base.amount, 100);
    assert_eq!(notes_1[0].base.asset, asset);

    let notes_2 = tint_provider.spendable_notes(account_2.receiver());
    assert_eq!(notes_2.len(), 1);
    assert_eq!(notes_2[0].base.amount, amount - 100);
    assert_eq!(notes_2[0].base.asset, asset);
}

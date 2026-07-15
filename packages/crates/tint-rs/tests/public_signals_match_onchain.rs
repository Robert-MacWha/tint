//! Cross-checks that the public-input vector `Provider::public_inputs`
//! computes locally matches what `Tint.computePublicSignals` computes
//! on-chain for the same operation -- independent of proof generation, so a
//! mismatch (e.g. an asset-encoding bug) shows up as a precise, labeled diff
//! instead of an opaque `InvalidProof` revert.

mod common;

use std::sync::Arc;

use alloy_primitives::{Address, B256, U256};
use ark_bn254::Fr;
use ark_ff::PrimeField;
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

use crate::common::anvil::{self};

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn public_signals_match_onchain() {
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
    let keys = Keys::from_seed(&[11u8; 32]);
    let account = Account::new(keys, Address::ZERO, B256::ZERO);

    let syncer = Arc::new(RpcSyncer::new(provider.clone(), *tint.address()));
    let verifier = Arc::new(RpcVerifier::new(provider.clone(), *tint.address()));
    let database = Arc::new(MemoryDatabase::default());
    let indexer = Indexer::new(syncer, verifier, database);
    let mut tint_provider = Provider::new(indexer, proving_key, verifying_key);
    tint_provider.add_account(account.clone());

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
        .deposit(account.receiver(), asset, amount, &mut rng)
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

    // Verify balances
    info!("Verifying balances");
    assert_eq!(
        token.balanceOf(*tint.address()).call().await.unwrap(),
        U256::from(amount)
    );

    let notes = tint_provider.spendable_notes(account.receiver());
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].base.amount, amount);
    assert_eq!(notes[0].base.asset, asset);

    let (call, local_signals) = tint_provider
        .public_inputs(
            [notes[0].clone()],
            [(account.receiver(), asset, amount)],
            [],
            &mut rng,
        )
        .unwrap();

    let onchain_signals = tint.call_builder(&call).call().await.unwrap();

    public_signal_diff(&local_signals, &onchain_signals);
}

fn public_signal_diff(local: &[Fr], onchain: &[U256]) {
    assert_eq!(local.len(), onchain.len(), "public signal length mismatch");
    let mut mismatch_found = false;
    for i in 0..local.len() {
        let local_fr = local[i];
        let onchain_fr = u256_to_fr(onchain[i]);
        if local_fr != onchain_fr {
            mismatch_found = true;
            println!(
                "Mismatch at index {}: local={}, onchain={}",
                i, local_fr, onchain_fr
            );
        }
    }

    if mismatch_found {
        panic!("Public signal mismatch found. See above for details.");
    } else {
        println!("All public signals match between local and on-chain computation.");
    }
}

fn u256_to_fr(u: U256) -> Fr {
    Fr::from_le_bytes_mod_order(&u.as_le_bytes())
}

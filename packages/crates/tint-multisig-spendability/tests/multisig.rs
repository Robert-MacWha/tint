mod common;

use std::sync::Arc;

use alloy_primitives::U256;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use k256::{ecdsa::SigningKey, elliptic_curve::Generate};
use tint::{
    account::{Account, keys::Keys, spending::NoopSpendingAccount},
    circuit::join_split::JoinSplitCircuit,
    database::memory::MemoryDatabase,
    indexer::{Indexer, syncer::RpcSyncer, verifier::RpcVerifier},
    note::asset::AssetId,
    provider::Provider,
};
use tint_multisig_spendability::{
    N_SIGNERS, THRESHOLD,
    account::{MultisigSpendingAccount, Signer},
    ffi::{self},
};
use tracing::info;

use crate::common::anvil;

/// Test that we can spend a note using the `MultisigSpendability` circuit,
/// including on-chain verification via the generated (uncompressed) Solidity
/// verifier.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn multisig() {
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
    let spendability = instance.spendability;

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts =
        tint_groth16::artifacts::Artifacts::generate_deterministic::<JoinSplitCircuit>().unwrap();

    // Setup spendability account
    info!("Setting up spendability account...");
    let signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS] =
        std::array::from_fn(|_| Box::new(SigningKey::generate()) as Box<dyn Signer + Send + Sync>);
    let spending = MultisigSpendingAccount::<N_SIGNERS, THRESHOLD>::new(
        *spendability.address(),
        signers,
        ffi::artifacts::load_artifacts().unwrap(),
    )
    .unwrap();
    let account_1 = Account::from_keys(Keys::from_seed(&[11u8; 32]), spending);
    let account_2 = Account::from_keys(Keys::from_seed(&[22u8; 32]), NoopSpendingAccount);

    // Setup a tint provider
    info!("Setting up tint provider...");
    let syncer = Arc::new(RpcSyncer::new(provider.clone(), *tint.address()));
    let verifier = Arc::new(RpcVerifier::new(provider.clone(), *tint.address()));
    let database = Arc::new(MemoryDatabase::default());
    let indexer = Indexer::new(syncer, verifier, database).await.unwrap();

    let mut tint_provider = Provider::new(indexer, artifacts);
    tint_provider.add_account(account_1.clone()).await.unwrap();
    tint_provider.add_account(account_2.clone()).await.unwrap();

    // Approve Tint to pull the deposits.
    let _ = token
        .approve(*tint.address(), U256::MAX)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Deposit into Tint.
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

    let notes = tint_provider.notes(account_1.receiver());

    // Spend the note, which is only spendable using the `MultisigSpendability` contract / circuit.
    info!("Spending note");
    let call = tint_provider
        .operate(
            [*notes[0]],
            [
                (account_1.receiver(), asset, amount - 100),
                (account_2.receiver(), asset, 100),
            ],
            [],
            &mut rng,
        )
        .await
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
    let notes_1 = tint_provider.notes(account_1.receiver());
    assert_eq!(notes_1.len(), 1);
    assert_eq!(notes_1[0].inner.amount, amount - 100);
    assert_eq!(notes_1[0].inner.asset, asset);

    let notes_2 = tint_provider.notes(account_2.receiver());
    assert_eq!(notes_2.len(), 1);
    assert_eq!(notes_2[0].inner.amount, 100);
    assert_eq!(notes_2[0].inner.asset, asset);
}

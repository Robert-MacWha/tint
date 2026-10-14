mod common;

use alloy_provider::Provider;
use alloy_rpc_types_eth::TransactionRequest;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use k256::{ecdsa::SigningKey, elliptic_curve::Generate};
use tint::{
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    deposit::Deposit,
    note::asset::AssetId,
    op::OperationBuilder,
    policy::Policy,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Syncer, rpc::RpcSyncer},
};
use tint_multisig_spendability::{
    N_SIGNERS, THRESHOLD,
    ffi::{self},
    policy::{MultisigPolicy, Signer},
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
    let tint = *instance.tint.address();
    let token = instance.token;
    let spendability = instance.spendability;

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts =
        tint_groth16::artifacts::Artifacts::generate_deterministic::<JoinSplitCircuit>().unwrap();

    // Setup policy
    info!("Setting up multisig policy...");
    let signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS] =
        std::array::from_fn(|_| Box::new(SigningKey::generate()) as Box<dyn Signer + Send + Sync>);
    let multisig = MultisigPolicy::<N_SIGNERS, THRESHOLD>::new(
        *spendability.address(),
        signers,
        ffi::artifacts::load_artifacts().unwrap(),
    )
    .unwrap();
    let account_1 = Account::from_seed(&[11u8; 32], multisig.policy());
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    // Deposit into Tint.
    info!("Depositing into Tint");
    let asset = AssetId::from(*token.address());
    let amount = 1_000u128;

    let deposit = Deposit::new(tint, account_1.pub_account(), asset, amount, &mut rng);
    send(&provider, deposit.approval()).await;
    send(&provider, deposit.request(&mut rng).unwrap()).await;

    info!("Syncing");
    let syncer = RpcSyncer::new(provider.clone());
    let snapshot = syncer.sync(tint, ..).await.unwrap();
    let mut mmr = SkewMmr::new();
    mmr.append_events(&snapshot.events).unwrap();

    let notes = snapshot.unspent(&account_1);

    // Spend the note, which is only spendable using the `MultisigSpendability` contract / circuit.
    info!("Spending note");
    let op = OperationBuilder::new(tint)
        .spend(notes[0].clone())
        .output(account_1.pub_account(), asset, amount - 100)
        .output(account_2.pub_account(), asset, 100)
        .build(&mut rng)
        .unwrap();
    let policy_input = multisig.authorize(&op).unwrap();
    let proven = op
        .authorize(vec![policy_input])
        .unwrap()
        .prove_with_mmr(&mmr, &artifacts, &mut rng)
        .unwrap();
    send(&provider, proven.into()).await;

    info!("Syncing");
    let snapshot = syncer.sync(tint, ..).await.unwrap();

    // Verify balances
    info!("Verifying balances");
    let notes_1 = snapshot.unspent(&account_1);
    assert_eq!(notes_1.len(), 1);
    assert_eq!(notes_1[0].note.amount, amount - 100);
    assert_eq!(notes_1[0].note.asset, asset);

    let notes_2 = snapshot.unspent(&account_2);
    assert_eq!(notes_2.len(), 1);
    assert_eq!(notes_2[0].note.amount, 100);
    assert_eq!(notes_2[0].note.asset, asset);
}

async fn send(provider: &impl Provider, tx: TransactionRequest) {
    let receipt = provider
        .send_transaction(tx)
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    assert!(receipt.status());
}

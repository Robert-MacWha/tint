mod common;

use alloy_node_bindings::Anvil;
use alloy_primitives::{B256, U256, hex};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolError;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use k256::{ecdsa::SigningKey, elliptic_curve::Generate};
use tint::{
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    deposit::Deposit,
    note::{asset::AssetId, owned::OwnedNote},
    operation::OperationBuilder,
    policy::{Authorizer, Policy},
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Syncer, rpc::RpcSyncer},
};
use tint_multisig_policy::{
    N_SIGNERS, THRESHOLD,
    ffi::{self},
    policy::{MultisigPolicy, Signer},
};
use tracing::info;

use crate::common::{
    anvil::{self, send},
    policy::{MultisigPolicy as MultisigPolicyContract, MultisigVerifier, deploy_policy},
};

/// Test that we can spend a note using the `MultisigPolicyCircuit`,
/// including on-chain verification via the generated (uncompressed) Solidity
/// verifier.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn multisig() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse()?)
        .add_directive("r1cs=off".parse()?);

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = anvil::setup().await?;
    let policy = deploy_policy(&instance.provider).await?;

    let provider = instance.provider;
    let tint = *instance.tint.address();
    let token = instance.token;

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts =
        tint_groth16::artifacts::Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    // Setup policy
    info!("Setting up multisig policy...");
    let signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS] =
        std::array::from_fn(|_| Box::new(SigningKey::generate()) as Box<dyn Signer + Send + Sync>);
    let multisig = MultisigPolicy::<N_SIGNERS, THRESHOLD>::new(
        policy,
        signers,
        ffi::artifacts::load_artifacts()?,
    )?;
    let account_1 = Account::from_seed(&[11u8; 32], multisig.policy());
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    // Deposit into Tint.
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

    let notes = snapshot.unspent(&account_1);

    // Spend the note, which is only spendable using the `MultisigPolicy` contract / circuit.
    info!("Spending note");
    let op = OperationBuilder::new()
        .spend(notes[0].clone())
        .output(account_1.pub_account(), asset, amount - 100)
        .output(account_2.pub_account(), asset, 100)
        .build(&mut rng)?;
    let policy_input = multisig.prove(&op)?;
    let proven = op
        .authorize(vec![policy_input])?
        .prove_with_mmr(&mmr, &artifacts, &mut rng)?;
    send(&provider, proven.transaction(tint)).await?;

    info!("Syncing");
    let snapshot = syncer.sync(tint, ..).await?;

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

    Ok(())
}

/// The placeholder proof should pass the verifier's commitment check and fail
/// only at the final pairing, so simulating with it costs as much gas as a
/// real proof.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn placeholder_reaches_pairing() -> Result<(), Box<dyn std::error::Error>> {
    let anvil = Anvil::new().spawn();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes())?;
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(anvil.endpoint().parse()?)
        .erased();
    let policy = deploy_policy(&provider).await?;
    let verifier = MultisigPolicyContract::new(policy, &provider)
        .VERIFIER()
        .call()
        .await?;

    let signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS] =
        std::array::from_fn(|_| Box::new(SigningKey::generate()) as Box<dyn Signer + Send + Sync>);
    let multisig = MultisigPolicy::<N_SIGNERS, THRESHOLD>::new(
        policy,
        signers,
        ffi::artifacts::load_artifacts()?,
    )?;

    let account = Account::from_seed(&[11u8; 32], multisig.policy());
    let note = account
        .pub_account()
        .commitment(AssetId::default(), 100, B256::ZERO);
    let op = OperationBuilder::new()
        .spend(OwnedNote::new(note, &account)?)
        .change_to(account.pub_account())
        .build(&mut StdRng::seed_from_u64(1))?;
    let placeholder = multisig.placeholder(&op).remove(0);

    let result = MultisigVerifier::new(verifier, &provider)
        .verifyProof(placeholder, [U256::ZERO, U256::ZERO])
        .call()
        .await;

    let Err(err) = result else {
        return Err("placeholder proof verified".into());
    };
    let selector = hex::encode(MultisigVerifier::ProofInvalid::SELECTOR);
    assert!(err.to_string().contains(&selector), "{err}");

    Ok(())
}

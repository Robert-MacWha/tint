//! Cross-checks that the public-signal vector proven locally matches what
//! `Tint.computePublicSignals` computes on-chain for the same operation.

mod common;

use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolCall;
use ark_bn254::Fr;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    abis::tint::Tint,
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    deposit::Deposit,
    fr::u256_to_fr,
    note::asset::AssetId,
    operation::OperationBuilder,
    policy::Policy,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Syncer, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;
use tracing::info;

use crate::common::anvil::{self, send};

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn public_signals_match_onchain() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse()?)
        .add_directive("r1cs=off".parse()?);

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = anvil::setup().await?;

    let provider = instance.provider;
    let tint = instance.tint;
    let token = instance.token;

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    let account = Account::from_seed(&[11u8; 32], Policy::NONE);

    // Deposit into Tint
    info!("Depositing into Tint");
    let asset = AssetId::from(*token.address());
    let amount = 1_000u128;

    let deposit = Deposit::new(
        *tint.address(),
        account.pub_account(),
        asset,
        amount,
        &mut rng,
    );
    send(&provider, deposit.approval()).await?;
    send(&provider, deposit.transaction(&mut rng)?).await?;

    info!("Syncing");
    let snapshot = RpcSyncer::new(provider.clone())
        .sync(*tint.address(), ..)
        .await?;
    let mut mmr = SkewMmr::new();
    mmr.append_events(&snapshot.events)?;

    // Verify balances
    info!("Verifying balances");
    assert_eq!(
        token.balanceOf(*tint.address()).call().await?,
        U256::from(amount)
    );

    let notes = snapshot.unspent(&account);
    assert_eq!(notes.len(), 1);

    let proven = OperationBuilder::new()
        .spend(notes[0].clone())
        .output(account.pub_account(), asset, amount - 100)
        .withdraw(Address::new([2; 20]), asset, 100)
        .build(&mut rng)?
        .authorize(vec![Bytes::default()])?
        .prove_with_mmr(&mmr, &artifacts, &mut rng)?;

    let call = Tint::computePublicSignalsCall::new((proven.operation().clone(),));
    let onchain_signals = tint.call_builder(&call).call().await?;

    public_signal_diff(proven.public_signals(), &onchain_signals)?;
    Ok(())
}

fn public_signal_diff(local: &[Fr], onchain: &[U256]) -> anyhow::Result<()> {
    assert_eq!(local.len(), onchain.len(), "public signal length mismatch");
    let mut mismatch_found = false;
    for i in 0..local.len() {
        let local_fr = local[i];
        let onchain_fr = u256_to_fr(onchain[i]);
        if local_fr != onchain_fr {
            mismatch_found = true;
            info!(
                "Mismatch at index {}: local={}, onchain={}",
                i, local_fr, onchain_fr
            );
        }
    }

    if mismatch_found {
        anyhow::bail!("Public signal mismatch found. See above for details.");
    }

    info!("All public signals match between local and on-chain computation.");
    Ok(())
}

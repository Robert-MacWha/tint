//! Cross-checks that the public-signal vector computed locally matches what
//! `ProofLib.toPublicSignals` computes on-chain for the same operation.

use alloy_primitives::{Address, B256, U256};
use alloy_sol_macro::sol;
use alloy_sol_types::SolValue;
use ark_bn254::Fr;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::{
    account::Account,
    fr::u256_to_fr,
    note::{asset::AssetId, owned::OwnedNote},
    operation::OperationBuilder,
    policy::Policy,
    skew_mmr::SkewMmr,
};
use tracing::info;

use crate::common::anvil::Instance;

mod common;

sol!(
    #[sol(rpc)]
    PublicSignals,
    "../../contracts/out/PublicSignals.sol/PublicSignals.json"
);

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn public_signals_match_onchain() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Instance::setup().await?;
    let provider = instance.provider.clone();

    let harness = PublicSignals::deploy(provider.clone()).await?;

    // "Spend" a note from a local MMR into an output and a withdrawal.
    let mut rng = StdRng::seed_from_u64(1);
    let account = Account::from_seed(&[11u8; 32], Policy::NONE);
    let asset = AssetId::default();
    let note = account.pub_account().commitment(asset, 1_000, B256::ZERO);
    let note = OwnedNote::new(note, &account)?;
    let mut mmr = SkewMmr::new();
    mmr.append(note.commitment());

    //? The public signals don't depend on the proof, so simulating computes
    //? them without proving.
    let simulated = OperationBuilder::new()
        .spend(note)
        .output(account.pub_account(), asset, 900)
        .withdraw(Address::repeat_byte(2), asset, 100)
        .build(&mut rng)?
        .simulate_with_mmr(&mmr)?;

    //? The harness's `sol!` binding has its own `Operation` type, so convert
    //? tint's through its ABI encoding.
    let operation = IPrivacyPool::Operation::abi_decode(&simulated.operation().abi_encode())?;
    let onchain_signals = harness.toPublicSignals(operation).call().await?;

    public_signal_diff(simulated.public_signals(), &onchain_signals)?;
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

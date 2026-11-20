use alloy_primitives::{Address, U256};
use ark_std::rand::{Rng, rngs::StdRng};
use rand_core::SeedableRng;
use tint::{
    account::Account, circuit::join_split::JoinSplitCircuit, fr::fr_to_b256,
    note::owned::OwnedNote, operation::OperationBuilder, policy::Policy, skew_mmr::SkewMmr,
};
use tint_groth16::artifacts::Artifacts;
use tracing::info;

use crate::common::{
    anvil::{Instance, send},
    util::{init_tracing, owned},
};

mod common;

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn unshield() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;

    let provider = instance.provider.clone();
    let tint = *instance.tint.address();
    let asset = instance.asset();

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    let account_1 = Account::from_seed(&[11u8; 32], Policy::NONE);
    let unshield_address = Address::new(rng.r#gen());

    let amount = 1_000u128;
    let (deposited, _) = instance
        .deposit(account_1.pub_account(), amount, &mut rng)
        .await?;

    let mut mmr = SkewMmr::new();
    mmr.append(deposited.hash());
    let note = OwnedNote::new(deposited, &account_1)?;

    // Unshield
    info!("Unshielding");
    let proven = OperationBuilder::new()
        .spend(note)
        .output(account_1.pub_account(), asset, 100)
        .withdraw(unshield_address, asset, amount - 100)
        .build(&mut rng)?
        .prove_with_mmr(&mmr, &artifacts, &mut rng)?;
    let unshield_receipt = send(&provider, proven.transaction(tint)).await?;
    info!("Unshielded for {} gas", unshield_receipt.gas_used);

    // Verify the note set and balances
    info!("Verifying note set and balances");
    for output in proven.outputs() {
        mmr.append(output.hash());
    }
    assert!(instance.verify_mmr(&mmr).await?);
    assert!(
        instance
            .tint
            .isSpent(fr_to_b256(proven.nullifiers()[0]))
            .call()
            .await?
    );
    assert_eq!(owned(proven.outputs(), &account_1), [(asset, 100)]);
    assert_eq!(
        instance.token.balanceOf(unshield_address).call().await?,
        U256::from(amount - 100)
    );

    Ok(())
}

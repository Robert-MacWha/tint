use alloy_primitives::U256;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    account::Account,
    deposit::Deposit,
    policy::Policy,
    syncer::{Syncer, rpc::RpcSyncer},
};
use tracing::info;

use crate::common::{
    anvil::{Instance, send},
    util::init_tracing,
};

mod common;

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn shield() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;

    let provider = instance.provider.clone();
    let tint = *instance.tint.address();
    let asset = instance.asset();

    let account = Account::from_seed(&[11u8; 32], Policy::NONE);

    // Deposit into Tint.
    info!("Depositing into Tint");
    let amount = 1_000u128;

    let deposit = Deposit::new(tint, account.pub_account(), asset, amount, &mut rng);
    send(&provider, deposit.approval()).await?;
    let shield_receipt = send(&provider, deposit.transaction(&mut rng)?).await?;
    info!("Shielded for {} gas", shield_receipt.gas_used);

    info!("Syncing");
    let snapshot = RpcSyncer::new(provider).sync(tint, ..).await?;

    // Verify balances
    info!("Verifying balances");
    assert_eq!(
        instance.token.balanceOf(tint).call().await?,
        U256::from(amount)
    );

    let notes = snapshot.unspent(&account);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].note, deposit.note());

    Ok(())
}

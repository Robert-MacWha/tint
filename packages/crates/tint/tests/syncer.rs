use alloy_primitives::U64;
use alloy_provider::Provider;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    account::Account,
    deposit::Deposit,
    note::asset::AssetId,
    policy::Policy,
    syncer::{Snapshot, Syncer, rpc::RpcSyncer},
};
use tracing::info;

use crate::common::anvil::{self, send};

mod common;

/// Walking backward through checkpoints recovers exactly the events in the
/// requested range, in emission order, regardless of the batch size.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn syncer() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = anvil::setup().await?;

    let provider = instance.provider;
    let tint = *instance.tint.address();
    let asset = AssetId::from(*instance.token.address());
    let account = Account::from_seed(&[11u8; 32], Policy::NONE);

    // Deposit in sparse blocks, separated by empty ones.
    info!("Depositing into Tint");
    let mut blocks = Vec::new();
    for _ in 0..4 {
        let _: () = provider
            .raw_request("anvil_mine".into(), (U64::from(20),))
            .await?;
        let deposit = Deposit::new(tint, account.pub_account(), asset, 1_000, &mut rng);
        send(&provider, deposit.approval()).await?;
        let receipt = send(&provider, deposit.transaction(&mut rng)?).await?;
        blocks.push(receipt.block_number.ok_or("receipt missing block number")?);
    }
    let _: () = provider
        .raw_request("anvil_mine".into(), (U64::from(20),))
        .await?;
    let latest = provider.get_block_number().await? + 1;

    for batch_size in [1, 5, 10_000] {
        info!("Syncing with batch size {batch_size}");
        let syncer = RpcSyncer::new(provider.clone()).with_batch_size(batch_size);

        let full = syncer.sync(tint, ..).await?;
        assert_eq!(full.range, 0..latest);
        assert_eq!(deposit_blocks(&full), blocks);

        let from = syncer.sync(tint, blocks[1]..).await?;
        assert_eq!(from.range, blocks[1]..latest);
        assert_eq!(deposit_blocks(&from), blocks[1..]);

        let to = syncer.sync(tint, ..blocks[3]).await?;
        assert_eq!(to.range, 0..blocks[3]);
        assert_eq!(deposit_blocks(&to), blocks[..3]);
    }

    Ok(())
}

fn deposit_blocks(snapshot: &Snapshot) -> Vec<u64> {
    snapshot
        .events
        .iter()
        .filter(|e| e.commitment().is_some())
        .map(tint::syncer::SyncEvent::block_number)
        .collect()
}

//! Runs the reference maker against a relay.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use alloy_primitives::{Address, B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use anyhow::Context;
use ark_bn254::Bn254;
use clap::Parser;
use rand_core::OsRng;
use reqwest::Url;
use tint::{account::Account, note::asset::AssetId, policy::Policy};
use tint_groth16::{
    artifacts::Artifacts,
    serde::{deserialize_matrices, deserialize_pk, deserialize_vk},
};
use tint_intent_policy::policy::IntentPolicy;
use tint_intent_relay::{client::RelayClient, maker::Maker};
use tracing::{error, info};

#[derive(Parser)]
struct Args {
    #[arg(long, env = "RPC_URL")]
    rpc_url: Url,
    #[arg(long, env = "PRIVATE_KEY")]
    private_key: PrivateKeySigner,
    #[arg(long, env = "TINT_ADDRESS")]
    tint: Address,
    #[arg(long, env = "INTENT_POLICY_ADDRESS")]
    policy: Address,
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    relay: Url,
    /// Seed of the maker's shielded account, whose notes are under no policy.
    #[arg(long, env = "MAKER_SEED")]
    seed: B256,
    /// An asset's value per base unit, as `ASSET=PRICE`. Repeatable.
    #[arg(long = "price", value_parser = parse_price)]
    prices: Vec<(AssetId, U256)>,
    /// Seconds between polls.
    #[arg(long, default_value_t = 10)]
    interval: u64,
    /// Directory of the join-split artifacts written by `gen_artifacts`.
    #[arg(long, default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/../tint/artifacts"))]
    artifacts: PathBuf,
    /// Directory of the intent policy artifacts written by its `gen_artifacts`.
    #[arg(
        long,
        default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/../tint-intent-policy/artifacts")
    )]
    policy_artifacts: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let provider = ProviderBuilder::new()
        .wallet(args.private_key)
        .connect_http(args.rpc_url)
        .erased();

    info!("Loading circuit artifacts...");
    let artifacts = load_artifacts(&args.artifacts)?;
    let policy_artifacts = load_artifacts(&args.policy_artifacts)?;

    let maker = Maker::new(
        Account::from_seed(&args.seed.0, Policy::NONE),
        args.prices.into_iter().collect::<HashMap<_, _>>(),
        args.tint,
        provider,
        RelayClient::new(args.relay),
        IntentPolicy::new(args.policy, policy_artifacts),
        artifacts,
    );

    loop {
        match maker.poll(&mut OsRng).await {
            Ok(settled) if !settled.is_empty() => info!(?settled, "Settled intents"),
            Ok(_) => {}
            Err(err) => error!("Poll failed: {err}"),
        }
        tokio::time::sleep(Duration::from_secs(args.interval)).await;
    }
}

fn parse_price(s: &str) -> anyhow::Result<(AssetId, U256)> {
    let (asset, price) = s.split_once('=').context("expected ASSET=PRICE")?;
    Ok((AssetId(asset.parse()?), price.parse()?))
}

fn load_artifacts(dir: &Path) -> anyhow::Result<Artifacts<Bn254>> {
    let read = |name: &str| fs::read(dir.join(name)).with_context(|| format!("reading {name}"));
    Ok(Artifacts {
        matrices: deserialize_matrices(&read("matrices.bin.br")?)?,
        pk: deserialize_pk(&read("proving_key.bin.br")?)?,
        vk: deserialize_vk(&read("verifying_key.bin.br")?)?,
    })
}

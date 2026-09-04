//! Cross-checks that hybrid compression (`ark_hybrid_compression_circuit::compress`)
//! computed locally matches what `ProofLib.toCompressedSignals` computes
//! on-chain for the same statement vector and `beta`.

mod common;

use alloy_node_bindings::Anvil;
use alloy_primitives::U256;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use ark_bn254::Fr;
use tint::{
    circuit::{join_split::N_PUB, poseidon2::crh::Poseidon2ChainCrh},
    fr::{fr_to_u256, u256_to_fr},
};
use tracing::info;

sol!(
    #[sol(rpc)]
    HybridCompression,
    "../../contracts/out/HybridCompression.sol/HybridCompression.json"
);

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn hybrid_compression_match_onchain() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let anvil = Anvil::new().spawn();
    let rpc_url = anvil.endpoint();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes()).unwrap();

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse().unwrap())
        .erased();

    let hybrid_compression = HybridCompression::deploy(provider.clone()).await.unwrap();

    let stmt: Vec<Fr> = (0..N_PUB).map(|i| Fr::from(i as u64 + 1)).collect();
    let (alpha, beta, gamma) =
        ark_hybrid_compression::circuit::compress::<Fr, Poseidon2ChainCrh>(&(), &stmt).unwrap();

    let sol_stmt: [U256; N_PUB] = std::array::from_fn(|i| fr_to_u256(stmt[i]));
    let (sol_alpha, sol_gamma) = {
        let compressed = hybrid_compression
            .toCompressedSignals(sol_stmt, fr_to_u256(beta))
            .call()
            .await
            .unwrap();
        (u256_to_fr(compressed[0]), u256_to_fr(compressed[2]))
    };

    info!(
        alpha = ?alpha, sol_alpha = ?sol_alpha,
        gamma = ?gamma, sol_gamma = ?sol_gamma,
        "hybrid compression local and on-chain outputs",
    );
    assert_eq!(alpha, sol_alpha, "alpha does not match on-chain value");
    assert_eq!(gamma, sol_gamma, "gamma does not match on-chain value");
}

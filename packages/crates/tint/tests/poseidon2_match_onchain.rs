//! Cross-checks that the poseidon2T2 and poseidon2T3 hash functions computed
//! locally match what `Tint.computePublicSignals` computes on-chain for the same
//! inputs.

mod common;

use alloy_node_bindings::Anvil;
use alloy_primitives::U256;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use tint::circuit::poseidon2::poseidon2_compress;
use tracing::info;

sol!(
    #[sol(rpc)]
    Poseidon2,
    "../../contracts/out/Poseidon2.sol/Poseidon2.json"
);

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn poseidon2_match_onchain() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse().unwrap())
        .add_directive("r1cs=off".parse().unwrap());

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let anvil = Anvil::new().spawn();
    let rpc_url = anvil.endpoint();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes()).unwrap();

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse().unwrap())
        .erased();

    let poseidon2 = Poseidon2::deploy(provider.clone()).await.unwrap();

    let t2_local_inputs = [Fr::from(1), Fr::from(2)];
    let t2_sol_inputs = [U256::from(1), U256::from(2)];

    let t2_local_output = poseidon2_compress(&t2_local_inputs);
    let t2_sol_output = poseidon2.poseidon2T2(t2_sol_inputs).call().await.unwrap();

    info!(
        t2_local_output = ?t2_local_output,
        t2_sol_output = ?t2_sol_output,
        "poseidon2T2 local and on-chain outputs",
    );
    assert_eq!(
        t2_local_output.into_bigint().to_bytes_le(),
        t2_sol_output.as_le_slice(),
        "poseidon2T2 local and on-chain outputs do not match"
    );

    let t3_local_inputs = [Fr::from(1), Fr::from(2), Fr::from(3)];
    let t3_sol_inputs = [U256::from(1), U256::from(2), U256::from(3)];

    let t3_local_output = poseidon2_compress(&t3_local_inputs);
    let t3_sol_output = poseidon2.poseidon2T3(t3_sol_inputs).call().await.unwrap();

    info!(
        t3_local_output = ?t3_local_output,
        t3_sol_output = ?t3_sol_output,
        "poseidon2T3 local and on-chain outputs",
    );
    assert_eq!(
        t3_local_output.into_bigint().to_bytes_le(),
        t3_sol_output.as_le_slice(),
        "poseidon2T3 local and on-chain outputs do not match"
    );
}

//! Cross-checks that the poseidon2T2 and poseidon2T3 hash functions computed
//! locally match the on-chain `Poseidon2T2` and `Poseidon2T3` contracts.

use alloy_node_bindings::Anvil;
use alloy_primitives::U256;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use tint_poseidon2::poseidon2_compress;
use tracing::info;

sol!(
    #[sol(rpc)]
    Poseidon2T2,
    "../../contracts/out/Poseidon2T2.sol/Poseidon2T2.json"
);

sol!(
    #[sol(rpc)]
    Poseidon2T3,
    "../../contracts/out/Poseidon2T3.sol/Poseidon2T3.json"
);

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn poseidon2_match_onchain() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse()?)
        .add_directive("r1cs=off".parse()?);

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let anvil = Anvil::new().spawn();
    let rpc_url = anvil.endpoint();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes())?;

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse()?)
        .erased();

    let poseidon2_t2 = Poseidon2T2::deploy(provider.clone()).await?;
    let poseidon2_t3 = Poseidon2T3::deploy(provider.clone()).await?;

    let t2_local_inputs = [Fr::from(1), Fr::from(2)];

    let t2_local_output = poseidon2_compress(&t2_local_inputs);
    let t2_sol_output = poseidon2_t2
        .compress(U256::from(1), U256::from(2), U256::ZERO)
        .call()
        .await?;

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

    let t3_local_output = poseidon2_compress(&t3_local_inputs);
    let t3_sol_output = poseidon2_t3
        .compress(U256::from(1), U256::from(2), U256::from(3), U256::ZERO)
        .call()
        .await?;

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

    Ok(())
}

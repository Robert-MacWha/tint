#![allow(dead_code)]

use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::{Address, B256};
use alloy_provider::{DynProvider, Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use anyhow::Context;
use tint::abis::tint::SIMULATION_SLOT;
use tracing::info;

use crate::common::anvil::{MockToken::MockTokenInstance, Tint::TintInstance};

pub struct Instance {
    // Anvil instance. Kills the anvil process when dropped.
    pub anvil: AnvilInstance,
    pub provider: DynProvider,
    pub tint: TintInstance<DynProvider>,
    pub token: MockTokenInstance<DynProvider>,
}

sol!(
    #[sol(rpc)]
    TintVerifier,
    "../../contracts/out/TintVerifier.sol/TintVerifier.json"
);

sol!(
    #[sol(rpc)]
    Poseidon2T3,
    "../../contracts/out/Poseidon2T3.sol/Poseidon2T3.json"
);

sol!(
    #[sol(rpc)]
    Tint,
    "../../contracts/out/Tint.sol/Tint.json"
);

sol!(
    #[sol(rpc)]
    MockToken,
    "../../contracts/out/MockToken.sol/MockToken.json"
);

/// Sets up an anvil instance for testing, deploying tint and a mock ERC20
pub async fn setup() -> anyhow::Result<Instance> {
    let anvil = Anvil::new().spawn();
    let rpc_url = anvil.endpoint();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes())?;

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse()?)
        .erased();

    let verifier = TintVerifier::deploy(provider.clone())
        .await
        .context("Error deploying verifier")?;
    let poseidon = Poseidon2T3::deploy(provider.clone()).await?;
    let tint = Tint::deploy(provider.clone(), *verifier.address(), *poseidon.address())
        .await
        .context("Error deploying tint")?;

    let token = MockToken::deploy(provider.clone()).await?;

    info!(
        verifier = ?verifier.address(),
        tint = ?tint.address(),
        token = ?token.address(),
        "Deployed contracts",
    );

    Ok(Instance {
        anvil,
        provider,
        tint,
        token,
    })
}

/// Sends `tx` and waits for a successful receipt.
pub async fn send(
    provider: &impl Provider,
    tx: alloy_rpc_types_eth::TransactionRequest,
) -> anyhow::Result<alloy_rpc_types_eth::TransactionReceipt> {
    let receipt = provider.send_transaction(tx).await?.get_receipt().await?;
    assert!(receipt.status());
    Ok(receipt)
}

/// Sets `tint`'s simulation flag, so it accepts operations assembled with
/// `simulate_with_mmr` in place of proven ones.
pub async fn enable_simulation(provider: &impl Provider, tint: Address) -> anyhow::Result<()> {
    let _: bool = provider
        .raw_request(
            "anvil_setStorageAt".into(),
            (tint, SIMULATION_SLOT, B256::with_last_byte(1)),
        )
        .await?;
    Ok(())
}

use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{DynProvider, Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use tracing::info;

use crate::common::anvil::{MockToken::MockTokenInstance, Tint::TintInstance};

#[allow(dead_code)]
pub struct Instance {
    // Anvil instance. Kills the anvil process when dropped.
    #[allow(dead_code)]
    pub anvil: AnvilInstance,
    pub provider: DynProvider,
    pub tint: TintInstance<DynProvider>,
    pub token: MockTokenInstance<DynProvider>,
}

sol!(
    #[sol(rpc)]
    Groth16Verifier,
    "../../contracts/out/Groth16Verifier.sol/Groth16Verifier.json"
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
#[allow(dead_code)]
pub async fn setup() -> anyhow::Result<Instance> {
    let anvil = Anvil::new().spawn();
    let rpc_url = anvil.endpoint();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes())?;

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse().unwrap())
        .erased();

    let verifier = Groth16Verifier::deploy(provider.clone()).await?;
    let tint = Tint::deploy(provider.clone(), verifier.address().clone()).await?;

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

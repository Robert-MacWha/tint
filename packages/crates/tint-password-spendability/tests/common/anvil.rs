use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{DynProvider, Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use tracing::info;

use crate::common::anvil::{
    MockToken::MockTokenInstance, Tint::TintInstance,
    spendability::PasswordSpendability::PasswordSpendabilityInstance,
};

#[allow(dead_code)]
pub struct Instance {
    // Anvil instance. Kills the anvil process when dropped.
    #[allow(dead_code)]
    pub anvil: AnvilInstance,
    pub provider: DynProvider,
    pub tint: TintInstance<DynProvider>,
    pub token: MockTokenInstance<DynProvider>,
    pub spendability: PasswordSpendabilityInstance<DynProvider>,
}

sol!(
    #[sol(rpc)]
    TintVerifier,
    "../../contracts/out/TintVerifier.sol/TintVerifier.json"
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

pub mod spendability {
    use alloy_sol_macro::sol;

    sol!(
        #[sol(rpc)]
        PasswordVerifier,
        "../../contracts/out/PasswordVerifier.sol/Verifier.json"
    );

    sol!(
        #[sol(rpc)]
        PasswordSpendability,
        "../../contracts/out/PasswordSpendability.sol/PasswordSpendability.json"
    );
}

/// Sets up an anvil instance for testing, deploying Tint, a mock ERC20, and
/// the PasswordSpendability contracts.
#[allow(dead_code)]
pub async fn setup() -> anyhow::Result<Instance> {
    let anvil = Anvil::new().spawn();
    let rpc_url = anvil.endpoint();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes())?;

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse().unwrap())
        .erased();

    let verifier = TintVerifier::deploy(provider.clone()).await?;
    let tint = Tint::deploy(provider.clone(), *verifier.address()).await?;
    let token = MockToken::deploy(provider.clone()).await?;

    let spendability_verifier = spendability::PasswordVerifier::deploy(provider.clone()).await?;
    let spendability = spendability::PasswordSpendability::deploy(
        provider.clone(),
        *spendability_verifier.address(),
    )
    .await?;

    info!(
        verifier = ?verifier.address(),
        tint = ?tint.address(),
        token = ?token.address(),
        spendability = ?spendability.address(),
        "Deployed contracts",
    );

    Ok(Instance {
        anvil,
        provider,
        tint,
        token,
        spendability,
    })
}

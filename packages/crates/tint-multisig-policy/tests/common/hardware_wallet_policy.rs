#![allow(dead_code)]

use alloy_primitives::Address;
use alloy_provider::DynProvider;
use alloy_sol_macro::sol;

use crate::common::policy::Poseidon2T2;

sol!(
    #[sol(rpc)]
    HardwareWalletVerifier,
    "../../contracts/out/HardwareWalletVerifier.sol/Verifier.json"
);

sol!(
    #[sol(rpc)]
    HardwareWalletPolicy,
    "../../contracts/out/HardwareWalletPolicy.sol/HardwareWalletPolicy.json"
);

/// Deploys a `HardwareWalletPolicy` with its verifier.
pub async fn deploy_policy(provider: &DynProvider) -> anyhow::Result<Address> {
    let verifier = HardwareWalletVerifier::deploy(provider.clone()).await?;
    let poseidon = Poseidon2T2::deploy(provider.clone()).await?;
    let policy =
        HardwareWalletPolicy::deploy(provider.clone(), *verifier.address(), *poseidon.address())
            .await?;

    Ok(*policy.address())
}

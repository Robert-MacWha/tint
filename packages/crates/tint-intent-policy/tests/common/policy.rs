use alloy_primitives::Address;
use alloy_provider::DynProvider;
use alloy_sol_macro::sol;

sol!(
    #[sol(rpc)]
    Poseidon2T2,
    "../../contracts/out/Poseidon2T2.sol/Poseidon2T2.json"
);

sol!(
    #[sol(rpc)]
    IntentVerifier,
    "../../contracts/out/IntentVerifier.sol/Verifier.json"
);

sol!(
    #[sol(rpc)]
    IntentPolicy,
    "../../contracts/out/IntentPolicy.sol/IntentPolicy.json"
);

/// Deploys an `IntentPolicy` with its verifier.
pub async fn deploy_policy(provider: &DynProvider) -> anyhow::Result<Address> {
    let verifier = IntentVerifier::deploy(provider.clone()).await?;
    let poseidon = Poseidon2T2::deploy(provider.clone()).await?;
    let policy =
        IntentPolicy::deploy(provider.clone(), *verifier.address(), *poseidon.address()).await?;

    Ok(*policy.address())
}

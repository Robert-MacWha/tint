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
    TimelockVerifier,
    "../../contracts/out/TimelockVerifier.sol/Verifier.json"
);

sol!(
    #[sol(rpc)]
    TimelockPolicy,
    "../../contracts/out/TimelockPolicy.sol/TimelockPolicy.json"
);

/// Deploys a `TimelockPolicy` with its verifier.
pub async fn deploy_policy(provider: &DynProvider) -> anyhow::Result<Address> {
    let verifier = TimelockVerifier::deploy(provider.clone()).await?;
    let poseidon = Poseidon2T2::deploy(provider.clone()).await?;
    let policy =
        TimelockPolicy::deploy(provider.clone(), *verifier.address(), *poseidon.address()).await?;

    Ok(*policy.address())
}

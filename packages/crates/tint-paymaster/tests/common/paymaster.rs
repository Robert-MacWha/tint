use alloy_primitives::{Address, U256};
use alloy_provider::DynProvider;
use alloy_sol_macro::sol;
use kohaku_fork_kit::{
    alto::{AltoBuilder, AltoInstance},
    entry_point::deploy_entry_point,
    simple_account::deploy_simple_account,
};
use kohaku_userop_kit::entry_point::ENTRY_POINT_08;
use tracing::info;

use crate::common::anvil::{Instance, send};

sol!(
    #[sol(rpc)]
    TintPaymaster,
    "../../contracts/out/TintPaymaster.sol/TintPaymaster.json"
);

const PAYMASTER_DEPOSIT: u128 = 10u128.pow(18);
const PAYMASTER_STAKE: u128 = 10u128.pow(18);
const UNSTAKE_DELAY_SEC: u32 = 3600;

const ALTO_EXECUTOR_PK: &str = "0x4a3a02862ddcb260ed52d40ef03f8e3d78fa3d174b0ef333afdf1ffb4a648cd5";
const ALTO_UTILITY_PK: &str = "0xdd4b2564c83ff7de602c39ffda1146055dc1814b07c083d7971722384f1f01a6";

/// Deploys a `TintPaymaster` over `instance` and starts a bundler for it.
///
/// Also deploys the `EntryPoint` and `SimpleAccount`, and uses the mock ERC20
/// as WETH.
pub async fn setup(
    instance: &Instance,
) -> anyhow::Result<(tint_paymaster::TintPaymaster, AltoInstance)> {
    let provider = &instance.provider;
    let weth = *instance.token.address();

    let entrypoint = deploy_entry_point(provider).await?;
    deploy_simple_account(provider).await?;
    let paymaster = tint_paymaster::TintPaymaster {
        address: deploy_paymaster(provider, *instance.tint.address(), weth).await?,
        weth,
    };

    info!("Starting local alto bundler");
    let alto = AltoBuilder::new(
        instance.anvil.endpoint(),
        entrypoint,
        ALTO_EXECUTOR_PK,
        ALTO_UTILITY_PK,
    )
    .prefund(provider)
    .await?
    .spawn()
    .await?;

    Ok((paymaster, alto))
}

/// Deploys a `TintPaymaster` over `tint` with a funded deposit and stake.
async fn deploy_paymaster(
    provider: &DynProvider,
    tint: Address,
    weth: Address,
) -> anyhow::Result<Address> {
    let paymaster = TintPaymaster::deploy(provider.clone(), ENTRY_POINT_08, tint, weth).await?;

    let deposit = paymaster.deposit().value(U256::from(PAYMASTER_DEPOSIT));
    send(provider, deposit.into_transaction_request()).await?;
    let stake = paymaster
        .addStake(UNSTAKE_DELAY_SEC)
        .value(U256::from(PAYMASTER_STAKE));
    send(provider, stake.into_transaction_request()).await?;

    Ok(*paymaster.address())
}

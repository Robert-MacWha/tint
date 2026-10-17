use alloy_primitives::{Address, U256};
use alloy_provider::DynProvider;
use alloy_sol_macro::sol;
use kohaku_userop_kit::entry_point::ENTRY_POINT_08;

use crate::common::anvil::send;

sol!(
    #[sol(rpc)]
    TintPaymaster,
    "../../contracts/out/TintPaymaster.sol/TintPaymaster.json"
);

const PAYMASTER_DEPOSIT: u128 = 10u128.pow(18);
const PAYMASTER_STAKE: u128 = 10u128.pow(18);
const UNSTAKE_DELAY_SEC: u32 = 3600;

/// Deploys a `TintPaymaster` over `tint`, funding its `EntryPoint` deposit and
/// stake.
pub async fn deploy_paymaster(
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

use alloy::{
    primitives::{Address, U256, aliases::U24},
    sol,
    sol_types::SolCall,
};
use rand_core::OsRng;
use tint::{abis::erc20::ERC20, note::asset::AssetId};
use tint_adapter::{Call, OperationBuilderAdapterExt, Shield, TintAdapter};

use crate::{
    account::Authorizer,
    chain::{self, Chain},
    config::StoredAccount,
};

sol! {
    interface ISwapRouter02 {
        struct ExactInputSingleParams {
            address tokenIn;
            address tokenOut;
            uint24 fee;
            address recipient;
            uint256 amountIn;
            uint256 amountOutMinimum;
            uint160 sqrtPriceLimitX96;
        }

        function exactInputSingle(ExactInputSingleParams calldata params) external payable returns (uint256 amountOut);
    }

    interface IAavePool {
        function supply(address asset, uint256 amount, address onBehalfOf, uint16 referralCode) external;
    }
}

/// Unshields `inputs` to the adapter, makes `calls`, and shields the adapter's
/// balance of each of `shields` back to `from`.
pub async fn relay(
    chain: &mut Chain,
    from: &StoredAccount,
    adapter: Address,
    inputs: Vec<(Address, u128)>,
    calls: Vec<Call>,
    shields: &[Address],
) -> anyhow::Result<()> {
    let account = from.account();
    let receiver = account.pub_account();
    let shields = shields
        .iter()
        .map(|&asset| Shield::new(asset, &receiver, &mut OsRng))
        .collect::<Result<_, _>>()?;
    let inputs = inputs
        .into_iter()
        .map(|(token, amount)| (AssetId::from(token), amount))
        .collect();
    let relay = TintAdapter { address: adapter }.relay(inputs, calls, shields);

    let op = chain::spend_builder(chain, &account, &relay.inputs)?
        .change_to(receiver)
        .with_relay(&relay)
        .build(&mut OsRng)?;
    let authorizer = Authorizer::load(from, chain.id).await?;
    let proven = chain::prove(chain, &authorizer, op).await?;
    chain.send(relay.transaction(&proven)?).await?;
    chain.sync().await
}

/// Calls swapping `amount` of `token_in` for `token_out` on Uniswap v3's `SwapRouter02`.
pub fn swap_calls(
    adapter: Address,
    router: Address,
    (token_in, token_out): (Address, Address),
    amount: u128,
    pool_fee: U24,
    min_out: u128,
) -> Vec<Call> {
    let swap = ISwapRouter02::ExactInputSingleParams {
        tokenIn: token_in,
        tokenOut: token_out,
        fee: pool_fee,
        recipient: adapter,
        amountIn: U256::from(amount),
        amountOutMinimum: U256::from(min_out),
        sqrtPriceLimitX96: U256::ZERO.to(),
    };
    vec![
        approve(token_in, router, amount),
        Call::new(
            router,
            ISwapRouter02::exactInputSingleCall::new((swap,)).abi_encode(),
        ),
    ]
}

/// Calls supplying `amount` of `token` to an Aave v3 pool on the adapter's behalf.
pub fn aave_supply_calls(
    adapter: Address,
    pool: Address,
    token: Address,
    amount: u128,
) -> Vec<Call> {
    let supply = IAavePool::supplyCall::new((token, U256::from(amount), adapter, 0));
    vec![
        approve(token, pool, amount),
        Call::new(pool, supply.abi_encode()),
    ]
}

fn approve(token: Address, spender: Address, amount: u128) -> Call {
    let approve = ERC20::approveCall {
        spender,
        amount: U256::from(amount),
    };
    Call::new(token, approve.abi_encode())
}

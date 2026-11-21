use alloy_node_bindings::Anvil;
use alloy_primitives::{Address, U256, address};
use alloy_provider::{DynProvider, Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use anyhow::Context;
use tracing::info;

use crate::common::anvil::{Instance, MockToken, Poseidon2T3, Tint, TintVerifier, send};

pub const WETH: Address = address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
pub const USDC: Address = address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
/// Uniswap v3 `SwapRouter02`.
pub const SWAP_ROUTER: Address = address!("0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45");
/// Aave v3 core `Pool`.
pub const AAVE_POOL: Address = address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2");
/// Aave v3 aToken for WETH.
pub const A_WETH: Address = address!("0x4d5F47FA6A74757f35C14fD3a6Ef8E3C9BC514E8");

/// Mainnet block the fork is pinned to.
const FORK_BLOCK: u64 = 26_466_000;
/// WETH the deployer wraps to deposit into Tint.
const WRAPPED: u128 = 10u128.pow(19);

sol! {
    #[sol(rpc)]
    interface IWETH {
        function deposit() external payable;
    }

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

sol!(
    #[sol(rpc)]
    TintAdapterContract,
    "../../contracts/out/TintAdapter.sol/TintAdapter.json"
);

/// Forks mainnet from `FORK_URL` and deploys Tint and a `TintAdapter` on it.
///
/// The instance's token is WETH, of which the deployer holds `WRAPPED`.
pub async fn setup() -> anyhow::Result<(Instance, tint_adapter::TintAdapter)> {
    let url = std::env::var("FORK_URL").context("FORK_URL must be a mainnet RPC URL")?;
    let anvil = Anvil::new()
        .fork(url)
        .fork_block_number(FORK_BLOCK)
        .timeout(60_000)
        .spawn();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes())?;
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(anvil.endpoint().parse()?)
        .erased();

    let verifier = TintVerifier::deploy(provider.clone()).await?;
    let poseidon = Poseidon2T3::deploy(provider.clone()).await?;
    let tint = Tint::deploy(provider.clone(), *verifier.address(), *poseidon.address()).await?;
    let adapter = TintAdapterContract::deploy(provider.clone(), *tint.address()).await?;
    wrap(&provider).await?;
    info!(tint = ?tint.address(), adapter = ?adapter.address(), "Deployed contracts");

    let instance = Instance {
        anvil,
        provider: provider.clone(),
        tint,
        token: MockToken::new(WETH, provider),
    };
    let adapter = tint_adapter::TintAdapter {
        address: *adapter.address(),
    };
    Ok((instance, adapter))
}

/// Wraps `WRAPPED` ETH into WETH for the provider's signer.
async fn wrap(provider: &DynProvider) -> anyhow::Result<()> {
    let weth = IWETH::new(WETH, provider);
    let deposit = weth.deposit().value(U256::from(WRAPPED));
    send(provider, deposit.into_transaction_request()).await?;
    Ok(())
}

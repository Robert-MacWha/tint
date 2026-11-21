use alloy_primitives::{Address, U256};
use alloy_sol_types::SolCall;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::{
    abis::erc20::ERC20, account::Account, circuit::join_split::JoinSplitCircuit,
    note::asset::AssetId, operation::OperationBuilder, policy::Policy,
    provider_ext::TintProviderExt,
};
use tint_adapter::{Call, OperationBuilderAdapterExt, Relay, Shield};
use tint_groth16::artifacts::Artifacts;
use tracing::info;

use crate::common::{
    anvil::{Instance, MockToken, send},
    fork::{self, A_WETH, AAVE_POOL, IAavePool, ISwapRouter02, SWAP_ROUTER, USDC, WETH},
    util::init_tracing,
};

mod common;

const NOTE_AMOUNT: u128 = 10u128.pow(18);
const RELAY_AMOUNT: u128 = NOTE_AMOUNT / 4;
/// WETH/USDC pool fee tier, in hundredths of a bip.
const POOL_FEE: u32 = 500;
/// A loose floor, in USDC base units. Real clients derive it from a quote.
const MIN_USDC_OUT: u128 = 10u128.pow(6);

/// Swaps a WETH note for a USDC note on Uniswap v3.
#[tokio::test]
#[ignore = "run with `FORK_URL=<mainnet rpc> cargo test --release -- --ignored`"]
async fn tint_adapter_uniswap_swap() -> anyhow::Result<()> {
    init_tracing();
    let mut rng = StdRng::seed_from_u64(1);
    let (instance, adapter) = fork::setup().await?;
    let account = Account::from_seed(&[11u8; 32], Policy::NONE);

    let swap = ISwapRouter02::ExactInputSingleParams {
        tokenIn: WETH,
        tokenOut: USDC,
        fee: POOL_FEE.try_into()?,
        recipient: adapter.address,
        amountIn: U256::from(RELAY_AMOUNT),
        amountOutMinimum: U256::from(MIN_USDC_OUT),
        sqrtPriceLimitX96: U256::ZERO.to(),
    };
    let relay = adapter.relay(
        vec![(AssetId::from(WETH), RELAY_AMOUNT)],
        vec![
            approve(WETH, SWAP_ROUTER, RELAY_AMOUNT),
            Call::new(
                SWAP_ROUTER,
                ISwapRouter02::exactInputSingleCall::new((swap,)).abi_encode(),
            ),
        ],
        vec![Shield::new(USDC, &account.pub_account(), &mut rng)?],
    );
    relay_note(&instance, &account, &relay, &mut rng).await?;

    let shielded = MockToken::new(USDC, &instance.provider)
        .balanceOf(*instance.tint.address())
        .call()
        .await?;
    assert!(
        shielded >= U256::from(MIN_USDC_OUT),
        "tint should hold the swapped USDC"
    );
    assert_owns(&instance, &account, USDC, |amount| {
        U256::from(amount) == shielded
    })
    .await
}

/// Supplies a WETH note to Aave v3, shielding the aWETH back as a note.
#[tokio::test]
#[ignore = "run with `FORK_URL=<mainnet rpc> cargo test --release -- --ignored`"]
async fn tint_adapter_aave_supply() -> anyhow::Result<()> {
    init_tracing();
    let mut rng = StdRng::seed_from_u64(1);
    let (instance, adapter) = fork::setup().await?;
    let account = Account::from_seed(&[11u8; 32], Policy::NONE);

    let supply = IAavePool::supplyCall::new((WETH, U256::from(RELAY_AMOUNT), adapter.address, 0));
    let relay = adapter.relay(
        vec![(AssetId::from(WETH), RELAY_AMOUNT)],
        vec![
            approve(WETH, AAVE_POOL, RELAY_AMOUNT),
            Call::new(AAVE_POOL, supply.abi_encode()),
        ],
        vec![Shield::new(A_WETH, &account.pub_account(), &mut rng)?],
    );
    relay_note(&instance, &account, &relay, &mut rng).await?;

    //? aToken balances round to the nearest wei of their scaled balance.
    assert_owns(&instance, &account, A_WETH, |amount| {
        amount.abs_diff(RELAY_AMOUNT) <= 1
    })
    .await
}

/// Deposits a `NOTE_AMOUNT` WETH note for `account` and relays it through `relay`.
async fn relay_note(
    instance: &Instance,
    account: &Account,
    relay: &Relay,
    rng: &mut StdRng,
) -> anyhow::Result<()> {
    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    instance
        .deposit(account.pub_account(), NOTE_AMOUNT, rng)
        .await?;
    let (snapshot, mmr) = instance.sync().await?;
    let notes = snapshot.unspent(account);

    info!("Relaying a note through the adapter");
    let operation = OperationBuilder::new(&account.encryption_key)
        .spend(notes[0].clone())
        .change_to(account.pub_account())
        .with_relay(relay)
        .build(rng)?
        .prove_with_mmr(&mmr, &artifacts, rng)?;
    send(&instance.provider, relay.transaction(&operation)?).await?;

    let spent = instance
        .provider
        .tint_is_spent(*instance.tint.address(), notes[0].nullifier())
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    assert!(spent, "note should be spent");
    Ok(())
}

/// Asserts `account` owns a note of `asset` whose amount satisfies `expected`.
async fn assert_owns(
    instance: &Instance,
    account: &Account,
    asset: Address,
    expected: impl Fn(u128) -> bool,
) -> anyhow::Result<()> {
    let (snapshot, _) = instance.sync().await?;
    let notes = snapshot.unspent(account);
    let shielded = notes
        .iter()
        .find(|n| n.note.asset == AssetId::from(asset))
        .ok_or_else(|| anyhow::anyhow!("account should own a shielded note"))?;
    assert!(expected(shielded.note.amount), "unexpected shielded amount");
    assert!(
        notes
            .iter()
            .any(|n| n.note.amount == NOTE_AMOUNT - RELAY_AMOUNT),
        "account should own its WETH change"
    );
    Ok(())
}

fn approve(token: Address, spender: Address, amount: u128) -> Call {
    let approve = ERC20::approveCall {
        spender,
        amount: U256::from(amount),
    };
    Call::new(token, approve.abi_encode())
}

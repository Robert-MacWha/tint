//! An unshield whose gas is paid from shielded WETH, against a live hegota devnet. Needs
//! `RPC_URL` and a funded `PRIVATE_KEY`, plus `just build-frame` in `packages/contracts`.

use std::time::Duration;

use alloy_hegota::{ExecutionStatus, FrameStatus, HegotaProviderExt, TxStatus};
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::{DynProvider, Provider, ProviderBuilder, network::TransactionBuilder};
use alloy_rpc_types_eth::{BlockNumberOrTag, TransactionRequest};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use alloy_sol_types::{SolCall, SolValue};
use anyhow::Context;
use ark_std::rand::{Rng, rngs::StdRng};
use rand_core::SeedableRng;
use tint::{
    abis::tint::Tint,
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    deposit::Deposit,
    fr::fr_to_b256,
    note::asset::AssetId,
    operation::OperationBuilder,
    policy::Policy,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Syncer, rpc::RpcSyncer},
};
use tint_frame::SponsoredOperation;
use tint_groth16::artifacts::Artifacts;
use tracing::info;

sol! {
    interface IWETH {
        function deposit() external payable;
        function balanceOf(address owner) external view returns (uint256);
    }
}

const FRAME_OPCODES: &str =
    include_str!("../../../contracts/out/FrameOpcodes.yul/FrameOpcodes.json");
const TINT_VERIFIER: &str =
    include_str!("../../../contracts/out/TintVerifier.sol/TintVerifier.json");
const POSEIDON: &str = include_str!("../../../contracts/out/Poseidon2T3.sol/Poseidon2T3.json");
const MOCK_WETH: &str = include_str!("../../../contracts/out/MockWETH.sol/MockWETH.json");
/// `TintFrame` with APPROVE patched in, from `just build-frame`.
const TINT_FRAME: &str = include_str!("../../../contracts/out/frame/TintFrame.hex");

const FLOAT: u128 = 50_000_000_000_000_000; // 0.05 ETH
const SHIELDED: u128 = 20_000_000_000_000_000; // 0.02 WETH
const UNSHIELDED: u128 = 1_000_000_000_000_000; // 0.001 WETH
const PRIORITY_FEE: u64 = 1_000_000_000;

struct Deployment {
    tint_frame: Address,
    weth: Address,
    block: u64,
}

#[tokio::test]
#[ignore = "needs a hegota devnet: RPC_URL=... PRIVATE_KEY=... cargo test --release -- --ignored"]
async fn sponsored_unshield() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,gr1cs=off,r1cs=off")
        .init();

    let signer: PrivateKeySigner = std::env::var("PRIVATE_KEY")?.parse()?;
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(std::env::var("RPC_URL")?.parse()?)
        .erased();
    let chain_id = provider.get_chain_id().await?;
    let mut rng = StdRng::seed_from_u64(1);

    let Deployment {
        tint_frame,
        weth,
        block,
    } = deploy(&provider).await?;
    let asset = AssetId::from(weth);

    info!("Shielding {SHIELDED} WETH");
    let account = Account::from_seed(&rng.r#gen::<[u8; 32]>(), Policy::NONE);
    send(
        &provider,
        call(weth, &IWETH::depositCall {}).with_value(U256::from(SHIELDED)),
    )
    .await?;
    let deposit = Deposit::new(tint_frame, account.pub_account(), asset, SHIELDED, &mut rng);
    send(&provider, deposit.approval()).await?;
    send(&provider, deposit.transaction(&mut rng)?).await?;

    let syncer = RpcSyncer::new(provider.clone());
    let snapshot = syncer.sync(tint_frame, block..).await?;
    let mut mmr = SkewMmr::new();
    mmr.append_events(&snapshot.events)?;
    let notes = snapshot.unspent(&account);
    assert_eq!(notes.len(), 1);

    info!("Proving a sponsored unshield");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;
    let base_fee = provider
        .get_block_by_number(BlockNumberOrTag::Latest)
        .await?
        .context("no latest block")?
        .header
        .base_fee_per_gas
        .context("no base fee")?;
    let max_fee = U256::from(2 * base_fee + PRIORITY_FEE);
    let sponsored = SponsoredOperation::new(
        tint_frame,
        weth,
        chain_id,
        max_fee,
        U256::from(PRIORITY_FEE),
    );
    info!("Fee: {} WETH wei", sponsored.fee()?);

    let recipient = Address::new(rng.r#gen());
    let proven = sponsored
        .prepare(
            OperationBuilder::new()
                .spend(notes[0].clone())
                .withdraw(recipient, asset, UNSHIELDED),
        )?
        .change_to(account.pub_account())
        .build(&mut rng)?
        .authorize(vec![Bytes::default()])?
        .prove_with_mmr(&mmr, &artifacts, &mut rng)?;
    let tx = sponsored.transaction(&proven)?;

    let simulation = provider.simulate_frame_transaction(&tx).await?;
    info!("Simulation: {simulation:?}");
    assert!(simulation.valid, "invalid: {:?}", simulation.violation);
    assert_eq!(simulation.execution_status, Some(ExecutionStatus::Success));

    info!("Sending");
    let float_before = provider.get_balance(tint_frame).await?;
    let hash = provider.send_frame_transaction(&tx).await?;
    let receipt = provider
        .watch_frame_receipt(hash, Duration::from_secs(120))
        .await?;
    info!("Receipt: {receipt:?}");

    assert_eq!(receipt.status, TxStatus::Succeeded);
    assert_eq!(receipt.payer, Some(tint_frame));
    assert!(
        receipt
            .frame_receipts
            .iter()
            .all(|f| f.status == FrameStatus::Succeeded)
    );

    let nullifier = fr_to_b256(proven.nullifiers()[0]);
    assert!(
        view(
            &provider,
            tint_frame,
            Tint::isSpentCall {
                nullifierHash: nullifier
            }
        )
        .await?
    );
    assert_eq!(
        view(&provider, weth, IWETH::balanceOfCall { owner: recipient }).await?,
        U256::from(UNSHIELDED)
    );
    // The fee covers max_cost, so the float only grows.
    assert!(provider.get_balance(tint_frame).await? >= float_before);
    Ok(())
}

/// Deploys `TintFrame` and its dependencies, and seeds its ETH float.
async fn deploy(provider: &DynProvider) -> anyhow::Result<Deployment> {
    info!("Deploying");
    let opcodes = create(provider, artifact(FRAME_OPCODES)?).await?.0;
    let verifier = create(provider, artifact(TINT_VERIFIER)?).await?.0;
    let poseidon = create(provider, artifact(POSEIDON)?).await?.0;
    let weth = create(provider, artifact(MOCK_WETH)?).await?.0;
    let args = (verifier, poseidon, weth, opcodes).abi_encode_params();
    let (tint_frame, block) = create(
        provider,
        [alloy_primitives::hex::decode(TINT_FRAME.trim())?, args].concat(),
    )
    .await?;
    info!(?tint_frame, ?weth, "Deployed");

    send(
        provider,
        TransactionRequest::default()
            .to(tint_frame)
            .value(U256::from(FLOAT)),
    )
    .await?;
    Ok(Deployment {
        tint_frame,
        weth,
        block,
    })
}

/// A forge artifact's creation bytecode.
fn artifact(json: &str) -> anyhow::Result<Vec<u8>> {
    let json: serde_json::Value = serde_json::from_str(json)?;
    let code = json["bytecode"]["object"].as_str().context("no bytecode")?;
    Ok(alloy_primitives::hex::decode(code)?)
}

async fn create(provider: &DynProvider, initcode: Vec<u8>) -> anyhow::Result<(Address, u64)> {
    let receipt = send(
        provider,
        TransactionRequest::default().with_deploy_code(initcode),
    )
    .await?;
    let address = receipt.contract_address.context("no contract address")?;
    Ok((address, receipt.block_number.context("no block number")?))
}

async fn send(
    provider: &DynProvider,
    tx: TransactionRequest,
) -> anyhow::Result<alloy_rpc_types_eth::TransactionReceipt> {
    let receipt = provider.send_transaction(tx).await?.get_receipt().await?;
    anyhow::ensure!(
        receipt.status(),
        "transaction {} reverted",
        receipt.transaction_hash
    );
    Ok(receipt)
}

fn call(to: Address, call: &impl SolCall) -> TransactionRequest {
    TransactionRequest::default()
        .to(to)
        .input(call.abi_encode().into())
}

async fn view<C: SolCall>(provider: &DynProvider, to: Address, c: C) -> anyhow::Result<C::Return> {
    let out = provider.call(call(to, &c)).await?;
    Ok(C::abi_decode_returns(&out)?)
}

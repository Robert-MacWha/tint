use alloy_primitives::{Address, U256, address};
use alloy_provider::Provider;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use kohaku_fork_kit::{
    alto::AltoBuilder, entry_point::deploy_entry_point, simple_account::deploy_simple_account,
};
use kohaku_userop_kit::{
    builder::UserOperationBuilder,
    bundler::Bundler,
    smart_account::simple_7702_smart_account::{Call, Simple7702SmartAccount},
};
use tint::{
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    deposit::Deposit,
    note::asset::AssetId,
    operation::OperationBuilder,
    policy::{NoPolicy, Policy},
    provider_ext::TintProviderExt,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Syncer, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;
use tint_paymaster::{TintPaymaster, UserOperationPaymasterExt};
use tracing::info;

use crate::common::{
    anvil::{self, MockToken, send},
    paymaster::deploy_paymaster,
};

mod common;

const ALTO_EXECUTOR_PK: &str = "0x4a3a02862ddcb260ed52d40ef03f8e3d78fa3d174b0ef333afdf1ffb4a648cd5";
const ALTO_UTILITY_PK: &str = "0xdd4b2564c83ff7de602c39ffda1146055dc1814b07c083d7971722384f1f01a6";
const SINK: Address = address!("0x000000000000000000000000000000000000dEaD");

const NOTE_AMOUNT: u128 = 10u128.pow(18);
const UNSHIELD_AMOUNT: u128 = NOTE_AMOUNT / 2;
const MAX_FEE: u128 = NOTE_AMOUNT / 10;

/// A userop paid for by a Tint note. The note's WETH is unshielded to the
/// sender during validation, so the userop's own calls can spend it.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn test_tint_paymaster() -> anyhow::Result<()> {
    init_tracing()?;
    let mut rng = StdRng::seed_from_u64(1);

    let instance = anvil::setup().await?;
    let provider = instance.provider.clone();
    let chain_id = provider.get_chain_id().await?;
    let tint = *instance.tint.address();
    //? `TintPaymaster` only treats WETH as an ERC20, so the mock token stands in.
    let weth = *instance.token.address();

    let entrypoint = deploy_entry_point(&provider).await?;
    deploy_simple_account(&provider).await?;
    let paymaster = TintPaymaster {
        address: deploy_paymaster(&provider, tint, weth).await?,
        tint,
        weth,
    };

    info!("Starting local alto bundler");
    let alto = AltoBuilder::new(
        instance.anvil.endpoint(),
        entrypoint,
        ALTO_EXECUTOR_PK,
        ALTO_UTILITY_PK,
    )
    .prefund(&provider)
    .await?
    .spawn()
    .await?;

    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    // Deposit a note
    info!("Depositing into Tint");
    let account = Account::from_seed(&[11u8; 32], Policy::NONE);
    let asset = AssetId::from(weth);
    let deposit = Deposit::new(tint, account.pub_account(), asset, NOTE_AMOUNT, &mut rng);
    send(&provider, deposit.approval()).await?;
    send(&provider, deposit.transaction(&mut rng)?).await?;

    // Sync a skew-MMR against the provider
    let syncer = RpcSyncer::new(provider.clone());
    let snapshot = syncer
        .sync(tint, ..)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    let mut mmr = SkewMmr::new();
    mmr.append_events(&snapshot.events)?;
    let notes = snapshot.unspent(&account);

    // Sponsor a UserOp with the note
    info!("Unshielding from Tint with the tint paymaster");
    let owner = PrivateKeySigner::random();
    let smart_account = Simple7702SmartAccount::new(provider.clone(), owner.address(), chain_id);

    let builder = UserOperationBuilder::new_with_smart_account(&smart_account)
        .await?
        .with_call(&vec![Call {
            target: weth,
            value: U256::ZERO,
            data: MockToken::transferCall::new((SINK, U256::from(UNSHIELD_AMOUNT)))
                .abi_encode()
                .into(),
        }]);

    let operation = OperationBuilder::new()
        .spend(notes[0].clone())
        .withdraw(owner.address(), asset, UNSHIELD_AMOUNT)
        .change_to(account.pub_account());
    let builder = builder
        .with_tint_paymaster(
            &paymaster, operation, &NoPolicy, &mmr, &artifacts, MAX_FEE, &*alto, &mut rng,
        )
        .await?;

    let userop = builder.build().sign(&owner).await?;
    let userop_hash = alto.send_user_operation(&userop).await?;
    let userop_receipt = alto.wait_for_receipt(userop_hash).await?;
    info!("Userop receipt: {userop_receipt:?}");

    assert!(userop_receipt.success, "userop should succeed");
    assert!(
        provider
            .tint_is_spent(tint, notes[0].nullifier())
            .await
            .map_err(|e| anyhow::anyhow!(e))?,
        "note should be spent"
    );

    let token = MockToken::new(weth, provider.clone());
    let fee = token.balanceOf(paymaster.address).call().await?;
    assert!(
        fee > U256::ZERO && fee <= U256::from(MAX_FEE),
        "paymaster should collect the fee"
    );
    assert_eq!(
        token.balanceOf(SINK).call().await?,
        U256::from(UNSHIELD_AMOUNT),
        "userop's call should spend the unshielded WETH"
    );

    Ok(())
}

fn init_tracing() -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse()?)
        .add_directive("r1cs=off".parse()?);
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    Ok(())
}

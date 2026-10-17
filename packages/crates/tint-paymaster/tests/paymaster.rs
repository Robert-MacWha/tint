use alloy_primitives::{Address, U256, address};
use alloy_provider::Provider;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use kohaku_fork_kit::{
    alto::{AltoBuilder, AltoInstance},
    entry_point::deploy_entry_point,
    simple_account::deploy_simple_account,
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
    policy::Policy,
    provider_ext::TintProviderExt,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Snapshot, Syncer, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;
use tint_paymaster::{Refund, TintPaymaster, UserOperationPaymasterExt};
use tracing::info;

use crate::common::{
    anvil::{self, Instance, MockToken, send},
    paymaster::deploy_paymaster,
};

mod common;

const ALTO_EXECUTOR_PK: &str = "0x4a3a02862ddcb260ed52d40ef03f8e3d78fa3d174b0ef333afdf1ffb4a648cd5";
const ALTO_UTILITY_PK: &str = "0xdd4b2564c83ff7de602c39ffda1146055dc1814b07c083d7971722384f1f01a6";
const SINK: Address = address!("0x000000000000000000000000000000000000dEaD");
const REFUND_RECIPIENT: Address = address!("0x000000000000000000000000000000000000fEEd");

const NOTE_AMOUNT: u128 = 10u128.pow(18);
const UNSHIELD_AMOUNT: u128 = NOTE_AMOUNT / 2;
const MAX_FEE: u128 = NOTE_AMOUNT / 10;

/// A userop paid for by a Tint note, refunding the unspent fee to an address.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn tint_paymaster_transfer_refund() -> anyhow::Result<()> {
    let mut rng = StdRng::seed_from_u64(1);
    let refund = Refund::transfer(REFUND_RECIPIENT);
    let sponsored = sponsor(refund, &mut rng).await?;

    let token = MockToken::new(
        sponsored.paymaster.weth,
        sponsored.instance.provider.clone(),
    );
    let kept = token.balanceOf(sponsored.paymaster.address).call().await?;
    let refunded = token.balanceOf(REFUND_RECIPIENT).call().await?;
    assert!(refunded > U256::ZERO, "recipient should be refunded");
    assert_eq!(
        kept + refunded,
        U256::from(MAX_FEE),
        "paymaster should keep only the fee it didn't refund"
    );

    Ok(())
}

/// A userop paid for by a Tint note, refunding the unspent fee as a new note.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn tint_paymaster_deposit_refund() -> anyhow::Result<()> {
    let mut rng = StdRng::seed_from_u64(1);
    let refund = Refund::deposit(&account().pub_account(), &mut rng)?;
    let sponsored = sponsor(refund, &mut rng).await?;

    let token = MockToken::new(
        sponsored.paymaster.weth,
        sponsored.instance.provider.clone(),
    );
    let kept = token.balanceOf(sponsored.paymaster.address).call().await?;
    let refunded = MAX_FEE - u128::try_from(kept)?;
    assert!(refunded > 0, "account should be refunded");

    let snapshot = sync(&sponsored.instance).await?;
    let notes = snapshot.unspent(&account());
    assert!(
        notes
            .iter()
            .any(|n| n.note.asset == AssetId::from(sponsored.paymaster.weth)
                && n.note.amount == refunded),
        "account should own the refunded note"
    );

    Ok(())
}

struct Sponsored {
    instance: Instance,
    paymaster: TintPaymaster,
    _alto: AltoInstance,
}

/// Deposits a note for [`account`] and spends it in a userop sponsored by a
/// fresh `TintPaymaster`, refunding to `refund`.
async fn sponsor(refund: Refund, rng: &mut StdRng) -> anyhow::Result<Sponsored> {
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
    let account = account();
    let asset = AssetId::from(weth);
    let deposit = Deposit::new(tint, account.pub_account(), asset, NOTE_AMOUNT, rng);
    send(&provider, deposit.approval()).await?;
    send(&provider, deposit.transaction(rng)?).await?;

    // Sync a skew-MMR against the provider
    let snapshot = sync(&instance).await?;
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
    let operation =
        builder.tint_paymaster_operation(&paymaster, operation, MAX_FEE, &refund, rng)?;
    // The note is under `Policy::NONE`, so there's nothing to authorize.
    let operation = operation.prove_with_mmr(&mmr, &artifacts, rng)?;
    let builder = builder
        .with_tint_paymaster(&paymaster, &operation, &refund, &*alto)
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
    assert_eq!(
        token.balanceOf(SINK).call().await?,
        U256::from(UNSHIELD_AMOUNT),
        "userop's call should spend the unshielded WETH"
    );

    Ok(Sponsored {
        instance,
        paymaster,
        _alto: alto,
    })
}

fn account() -> Account {
    Account::from_seed(&[11u8; 32], Policy::NONE)
}

async fn sync(instance: &Instance) -> anyhow::Result<Snapshot> {
    RpcSyncer::new(instance.provider.clone())
        .sync(*instance.tint.address(), ..)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

use alloy_primitives::{Address, U256, address};
use alloy_provider::Provider;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use kohaku_userop_kit::{
    builder::UserOperationBuilder,
    bundler::{Bundler, BundlerExt},
    smart_account::{
        SmartAccount,
        simple_7702_smart_account::{Call, Simple7702SmartAccount},
    },
};
use tint::{
    account::Account, circuit::join_split::JoinSplitCircuit, operation::OperationBuilder,
    policy::Policy, provider_ext::TintProviderExt,
};
use tint_groth16::artifacts::Artifacts;
use tint_paymaster::{OperationBuilderPaymasterExt, Refund, UserOperationPaymasterExt};
use tracing::info;

use crate::common::{
    anvil::{Instance, MockToken},
    paymaster,
};

mod common;

const SINK: Address = address!("0x000000000000000000000000000000000000dEaD");
const REFUND_RECIPIENT: Address = address!("0x000000000000000000000000000000000000fEEd");

const NOTE_AMOUNT: u128 = 10u128.pow(18);
const UNSHIELD_AMOUNT: u128 = NOTE_AMOUNT / 2;

/// A userop paid for by a Tint note, refunding the unspent fee to an address.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn tint_paymaster_transfer_refund() -> anyhow::Result<()> {
    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;
    let provider = instance.provider.clone();
    let chain_id = provider.get_chain_id().await?;
    let tint = *instance.tint.address();
    let weth = *instance.token.address();
    let asset = instance.asset();
    let token = &instance.token;
    let (paymaster, alto) = paymaster::setup(&instance).await?;

    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    let account = Account::from_seed(&[11u8; 32], Policy::NONE);
    instance
        .deposit(account.pub_account(), NOTE_AMOUNT, &mut rng)
        .await?;
    let (snapshot, mmr) = instance.sync().await?;
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

    // Estimate gas with a simulated operation paying a nominal fee
    let mut payment = paymaster.pay(
        1,
        Refund::transfer(REFUND_RECIPIENT),
        smart_account.address(),
    );
    let simulated = operation
        .clone()
        .with_paymaster(&payment)
        .build(&mut rng)?
        .simulate_with_mmr(&mmr)?;
    let builder = builder
        .with_tint_paymaster(&payment, &simulated)?
        .with_gas_estimate(&*alto, Some(&paymaster.simulation_override(tint)))
        .await?;

    // Pay the estimated fee with a proven operation
    payment.fee = builder.max_gas_cost();
    let operation = operation
        .with_paymaster(&payment)
        .build(&mut rng)?
        .prove_with_mmr(&mmr, &artifacts, &mut rng)?;

    let userop = builder
        .with_tint_paymaster(&payment, &operation)?
        .build()
        .sign(&owner)
        .await?;
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
    assert_eq!(
        token.balanceOf(SINK).call().await?,
        U256::from(UNSHIELD_AMOUNT),
        "userop's call should spend the unshielded WETH"
    );

    let kept = token.balanceOf(paymaster.address).call().await?;
    assert!(
        kept >= userop_receipt.actual_gas_cost,
        "paymaster should keep at least the gas it paid for"
    );
    let refunded = token.balanceOf(REFUND_RECIPIENT).call().await?;
    assert!(refunded > U256::ZERO, "recipient should be refunded");
    assert_eq!(
        kept + refunded,
        U256::from(payment.fee),
        "paymaster should keep only the fee it didn't refund"
    );

    Ok(())
}

/// A userop paid for by a Tint note, refunding the unspent fee as a new note.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn tint_paymaster_deposit_refund() -> anyhow::Result<()> {
    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;
    let provider = instance.provider.clone();
    let chain_id = provider.get_chain_id().await?;
    let tint = *instance.tint.address();
    let weth = *instance.token.address();
    let asset = instance.asset();
    let token = &instance.token;
    let (paymaster, alto) = paymaster::setup(&instance).await?;

    info!("Setting up circuits...");
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    let account = Account::from_seed(&[11u8; 32], Policy::NONE);
    instance
        .deposit(account.pub_account(), NOTE_AMOUNT, &mut rng)
        .await?;
    let (snapshot, mmr) = instance.sync().await?;
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

    // Estimate gas with a simulated operation paying a nominal fee
    let mut payment = paymaster.pay(
        1,
        Refund::deposit(&account.pub_account(), &mut rng)?,
        smart_account.address(),
    );
    let simulated = operation
        .clone()
        .with_paymaster(&payment)
        .build(&mut rng)?
        .simulate_with_mmr(&mmr)?;
    let builder = builder
        .with_tint_paymaster(&payment, &simulated)?
        .with_gas_estimate(&*alto, Some(&paymaster.simulation_override(tint)))
        .await?;

    // Pay the estimated fee with a proven operation
    payment.fee = builder.max_gas_cost();
    let operation = operation
        .with_paymaster(&payment)
        .build(&mut rng)?
        .prove_with_mmr(&mmr, &artifacts, &mut rng)?;

    let userop = builder
        .with_tint_paymaster(&payment, &operation)?
        .build()
        .sign(&owner)
        .await?;
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
    assert_eq!(
        token.balanceOf(SINK).call().await?,
        U256::from(UNSHIELD_AMOUNT),
        "userop's call should spend the unshielded WETH"
    );

    let kept = token.balanceOf(paymaster.address).call().await?;
    assert!(
        kept >= userop_receipt.actual_gas_cost,
        "paymaster should keep at least the gas it paid for"
    );
    let refunded = payment.fee - u128::try_from(kept)?;
    assert!(refunded > 0, "account should be refunded");

    let (snapshot, _) = instance.sync().await?;
    let notes = snapshot.unspent(&account);
    assert!(
        notes
            .iter()
            .any(|n| n.note.asset == asset && n.note.amount == refunded),
        "account should own the refunded note"
    );

    Ok(())
}

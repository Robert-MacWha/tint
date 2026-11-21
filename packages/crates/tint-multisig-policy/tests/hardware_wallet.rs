mod common;

use alloy_provider::Provider;
use alloy_signer_local::PrivateKeySigner;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::{
    account::Account, circuit::join_split::JoinSplitCircuit, operation::OperationBuilder,
    policy::Policy,
};
use tint_multisig_policy::{ffi, hardware_wallet::HardwareWalletPolicy};
use tracing::info;

use crate::common::{
    anvil::{Instance, send},
    hardware_wallet_policy::deploy_policy,
    util::{assert_unspent, init_tracing},
};

/// Test that we can spend a note using the `HardwareWalletPolicyCircuit`,
/// signed through the `alloy_signer::Signer` trait hardware wallets
/// implement, including on-chain verification via the generated Solidity
/// verifier.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn hardware_wallet() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;
    let policy = deploy_policy(&instance.provider).await?;
    let chain_id = instance.provider.get_chain_id().await?;

    let tint = *instance.tint.address();
    let asset = instance.asset();

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts =
        tint_groth16::artifacts::Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    // Setup policy
    info!("Setting up hardware wallet policy...");
    let hardware_wallet = HardwareWalletPolicy::new(
        policy,
        chain_id,
        PrivateKeySigner::random(),
        ffi::artifacts::load_hardware_wallet_artifacts()?,
    )
    .await?;
    let account_1 = Account::from_seed(&[11u8; 32], hardware_wallet.policy());
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    let amount = 1_000u128;
    instance
        .deposit(account_1.pub_account(), amount, &mut rng)
        .await?;

    let (snapshot, mmr) = instance.sync().await?;
    let notes = snapshot.unspent(&account_1);

    // Spend the note, which is only spendable using the `HardwareWalletPolicy` contract / circuit.
    info!("Spending note");
    let mut op = OperationBuilder::new(&account_1.encryption_key)
        .spend(notes[0].clone())
        .output(account_1.pub_account(), asset, amount - 100)
        .output(account_2.pub_account(), asset, 100)
        .build(&mut rng)?;
    hardware_wallet.authorize(&mut op).await?;
    let proven = op.prove_with_mmr(&mmr, &artifacts, &mut rng)?;
    send(&instance.provider, proven.transaction(tint)).await?;

    // Verify balances
    info!("Verifying balances");
    let (snapshot, _) = instance.sync().await?;
    assert_unspent(&snapshot, &account_1, &[(asset, amount - 100)]);
    assert_unspent(&snapshot, &account_2, &[(asset, 100)]);

    Ok(())
}

mod common;

use ark_std::rand::{SeedableRng, rngs::StdRng};
use k256::{ecdsa::SigningKey, elliptic_curve::Generate};
use tint::{
    account::Account, circuit::join_split::JoinSplitCircuit, operation::OperationBuilder,
    policy::Policy,
};
use tint_multisig_policy::{
    N_SIGNERS, THRESHOLD,
    ffi::{self},
    policy::{MultisigPolicy, Signer},
};
use tracing::info;

use crate::common::{
    anvil::{Instance, send},
    policy::deploy_policy,
    util::{assert_unspent, init_tracing},
};

/// Test that we can spend a note using the `MultisigPolicyCircuit`,
/// including on-chain verification via the generated (uncompressed) Solidity
/// verifier.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn multisig() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;
    let policy = deploy_policy(&instance.provider).await?;

    let tint = *instance.tint.address();
    let asset = instance.asset();

    // Setup circuits
    info!("Setting up circuits...");
    let artifacts =
        tint_groth16::artifacts::Artifacts::generate_deterministic::<JoinSplitCircuit>()?;

    // Setup policy
    info!("Setting up multisig policy...");
    let signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS] =
        std::array::from_fn(|_| Box::new(SigningKey::generate()) as Box<dyn Signer + Send + Sync>);
    let multisig = MultisigPolicy::<N_SIGNERS, THRESHOLD>::new(
        policy,
        signers,
        ffi::artifacts::load_artifacts()?,
    )?;
    let account_1 = Account::from_seed(&[11u8; 32], multisig.policy());
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);

    let amount = 1_000u128;
    instance
        .deposit(account_1.pub_account(), amount, &mut rng)
        .await?;

    let (snapshot, mmr) = instance.sync().await?;
    let notes = snapshot.unspent(&account_1);

    // Spend the note, which is only spendable using the `MultisigPolicy` contract / circuit.
    info!("Spending note");
    let mut op = OperationBuilder::new()
        .spend(notes[0].clone())
        .output(account_1.pub_account(), asset, amount - 100)
        .output(account_2.pub_account(), asset, 100)
        .build(&mut rng)?;
    multisig.authorize(&mut op)?;
    let proven = op.prove_with_mmr(&mmr, &artifacts, &mut rng)?;
    send(&instance.provider, proven.transaction(tint)).await?;

    // Verify balances
    info!("Verifying balances");
    let (snapshot, _) = instance.sync().await?;
    assert_unspent(&snapshot, &account_1, &[(asset, amount - 100)]);
    assert_unspent(&snapshot, &account_2, &[(asset, 100)]);

    Ok(())
}

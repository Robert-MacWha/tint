mod common;

use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tint::{
    account::Account, circuit::join_split::JoinSplitCircuit, operation::OperationBuilder,
    policy::Policy,
};
use tint_groth16::artifacts::Artifacts;
use tint_intent_policy::{
    circuit::IntentPolicyCircuit,
    intent::{CancelKey, Intent, OperationBuilderIntentExt},
    policy::IntentPolicy,
};
use tracing::info;

use crate::common::{
    anvil::{Instance, send},
    policy::deploy_policy,
    util::{assert_unspent, init_tracing},
};

const OFFER: u128 = 1_000;
const WANT: u128 = 600;

/// Test that a counterparty can spend an intent note by creating the wanted note.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn fill() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let setup = Setup::new(&mut rng).await?;
    let asset = setup.instance.asset();
    let maker = Account::from_seed(&[33u8; 32], Policy::NONE);

    setup
        .instance
        .deposit(maker.pub_account(), WANT, &mut rng)
        .await?;
    let (snapshot, mmr) = setup.instance.sync().await?;

    info!("Filling intent");
    let mut op = OperationBuilder::new(&maker.encryption_key)
        .fill(&setup.intent)
        .spend(snapshot.unspent(&maker)[0].clone())
        .change_to(maker.pub_account())
        .build(&mut rng)?;
    setup.policy.authorize_fill(&mut op, &setup.intent)?;
    let proven = op.prove_with_mmr(&mmr, &setup.artifacts, &mut rng)?;
    send(&setup.instance.provider, proven.transaction(setup.tint())).await?;

    let (snapshot, _) = setup.instance.sync().await?;
    let received = setup.intent.received(&snapshot, &setup.owner);
    assert_eq!(received.map(|n| n.note), Some(setup.intent.want));
    assert_unspent(&snapshot, &maker, &[(asset, OFFER)]);

    Ok(())
}

/// Test that an intent's owner can reclaim its note with the cancel key.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn cancel() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let setup = Setup::new(&mut rng).await?;
    let asset = setup.instance.asset();
    let (_, mmr) = setup.instance.sync().await?;

    info!("Cancelling intent");
    let mut op = OperationBuilder::new(&setup.owner.encryption_key)
        .spend(setup.intent.note())
        .output(setup.owner.pub_account(), asset, OFFER)
        .build(&mut rng)?;
    setup
        .policy
        .authorize_cancel(&mut op, &setup.intent, &setup.cancel_key)?;
    let proven = op.prove_with_mmr(&mmr, &setup.artifacts, &mut rng)?;
    send(&setup.instance.provider, proven.transaction(setup.tint())).await?;

    let (snapshot, _) = setup.instance.sync().await?;
    assert_unspent(&snapshot, &setup.owner, &[(asset, OFFER)]);

    Ok(())
}

/// A deployment with a funded intent offering `OFFER` for `WANT` of the mock token.
struct Setup {
    instance: Instance,
    artifacts: Artifacts<ark_bn254::Bn254>,
    policy: IntentPolicy,
    owner: Account,
    intent: Intent,
    cancel_key: CancelKey,
}

impl Setup {
    async fn new(rng: &mut StdRng) -> anyhow::Result<Self> {
        let instance = Instance::setup().await?;
        let address = deploy_policy(&instance.provider).await?;
        let asset = instance.asset();

        info!("Setting up circuits...");
        let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;
        let policy = IntentPolicy::new(
            address,
            Artifacts::generate_deterministic::<IntentPolicyCircuit>()?,
        );

        let owner = Account::from_seed(&[11u8; 32], Policy::NONE);
        instance.deposit(owner.pub_account(), OFFER, rng).await?;
        let (snapshot, mmr) = instance.sync().await?;

        info!("Funding intent");
        let expiry = u64::MAX;
        let (intent, cancel_key) = Intent::new(
            address,
            (asset, OFFER),
            (asset, WANT),
            &owner.pub_account(),
            expiry,
            rng,
        )?;
        let op = OperationBuilder::new(&owner.encryption_key)
            .spend(snapshot.unspent(&owner)[0].clone())
            .output_note(intent.offer)
            .build(rng)?;
        let proven = op.prove_with_mmr(&mmr, &artifacts, rng)?;
        send(
            &instance.provider,
            proven.transaction(*instance.tint.address()),
        )
        .await?;

        Ok(Self {
            instance,
            artifacts,
            policy,
            owner,
            intent,
            cancel_key,
        })
    }

    fn tint(&self) -> alloy_primitives::Address {
        *self.instance.tint.address()
    }
}

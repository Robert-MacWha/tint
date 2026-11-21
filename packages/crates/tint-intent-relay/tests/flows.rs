//! The reference intent flows, settled by the reference maker through the relay.

mod common;

use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use alloy_primitives::U256;
use ark_bn254::Bn254;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::{
    account::Account, circuit::join_split::JoinSplitCircuit, deposit::Deposit,
    note::asset::AssetId, operation::OperationBuilder, policy::Policy, skew_mmr::SkewMmr,
    syncer::Snapshot,
};
use tint_groth16::artifacts::Artifacts;
use tint_intent_policy::{circuit::IntentPolicyCircuit, intent::Intent, policy::IntentPolicy};
use tint_intent_relay::{client::RelayClient, maker::Maker};
use tracing::info;

use crate::common::{
    anvil::{Instance, MockToken, send},
    policy::deploy_policy,
    spawn_relay,
    util::{assert_unspent, init_tracing},
};

/// A user offers 1000 A for 600 B; a maker fills it from its own B.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn limit_order() -> anyhow::Result<()> {
    init_tracing();
    let mut rng = StdRng::seed_from_u64(1);
    let env = Env::setup().await?;
    let user = Account::from_seed(&[11u8; 32], Policy::NONE);
    let maker_account = Account::from_seed(&[33u8; 32], Policy::NONE);

    env.deposit(env.a, &user, 1_000, &mut rng).await?;
    env.deposit(env.b, &maker_account, 1_000, &mut rng).await?;
    let intent = env
        .post(&user, (env.a, 1_000), (env.b, 600), &mut rng)
        .await?;

    let prices = HashMap::from([(env.a, U256::from(1)), (env.b, U256::from(1))]);
    let settled = env
        .maker(maker_account.clone(), prices)
        .poll(&mut rng)
        .await?;
    assert_eq!(settled, vec![intent.id()]);

    let (snapshot, _) = env.instance.sync().await?;
    assert_received(&snapshot, &intent, &user);
    assert_unspent(&snapshot, &maker_account, &[(env.a, 1_000), (env.b, 400)]);
    Ok(())
}

/// Two users post opposite intents; a maker matches them against each other,
/// keeping the surplus.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn p2p_swap() -> anyhow::Result<()> {
    init_tracing();
    let mut rng = StdRng::seed_from_u64(1);
    let env = Env::setup().await?;
    let alice = Account::from_seed(&[11u8; 32], Policy::NONE);
    let bob = Account::from_seed(&[22u8; 32], Policy::NONE);
    let matcher = Account::from_seed(&[33u8; 32], Policy::NONE);

    env.deposit(env.a, &alice, 1_000, &mut rng).await?;
    env.deposit(env.b, &bob, 500, &mut rng).await?;
    let alice_intent = env
        .post(&alice, (env.a, 1_000), (env.b, 400), &mut rng)
        .await?;
    let bob_intent = env.post(&bob, (env.b, 500), (env.a, 900), &mut rng).await?;

    let settled = env
        .maker(matcher.clone(), HashMap::new())
        .poll(&mut rng)
        .await?;
    assert_eq!(settled, vec![alice_intent.id(), bob_intent.id()]);

    let (snapshot, _) = env.instance.sync().await?;
    assert_received(&snapshot, &alice_intent, &alice);
    assert_received(&snapshot, &bob_intent, &bob);
    assert_unspent(&snapshot, &matcher, &[(env.a, 100), (env.b, 100)]);
    Ok(())
}

/// A Tint deployment with two tokens, an `IntentPolicy`, and a relay.
struct Env {
    instance: Instance,
    relay: RelayClient,
    policy: IntentPolicy,
    artifacts: Artifacts<Bn254>,
    a: AssetId,
    b: AssetId,
}

impl Env {
    async fn setup() -> anyhow::Result<Self> {
        let instance = Instance::setup().await?;
        let policy_address = deploy_policy(&instance.provider).await?;
        let token_b = MockToken::deploy(instance.provider.clone()).await?;

        info!("Setting up circuits...");
        let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>()?;
        let policy_artifacts = Artifacts::generate_deterministic::<IntentPolicyCircuit>()?;

        Ok(Self {
            a: instance.asset(),
            b: AssetId::from(*token_b.address()),
            instance,
            relay: spawn_relay().await?,
            policy: IntentPolicy::new(policy_address, policy_artifacts),
            artifacts,
        })
    }

    /// Deposits `amount` of `asset` as a note for `owner`.
    async fn deposit(
        &self,
        asset: AssetId,
        owner: &Account,
        amount: u128,
        rng: &mut StdRng,
    ) -> anyhow::Result<()> {
        let tint = *self.instance.tint.address();
        let deposit = Deposit::new(tint, owner.pub_account(), asset, amount, rng);
        send(&self.instance.provider, deposit.approval()).await?;
        send(&self.instance.provider, deposit.transaction(rng)?).await?;
        Ok(())
    }

    /// Moves `owner`'s only note into an intent for `want`, and posts it.
    async fn post(
        &self,
        owner: &Account,
        offer: (AssetId, u128),
        want: (AssetId, u128),
        rng: &mut StdRng,
    ) -> anyhow::Result<Intent> {
        let expiry = now() + 3_600;
        let (intent, _) = Intent::new(
            self.policy.address(),
            offer,
            want,
            &owner.pub_account(),
            expiry,
            rng,
        )?;

        let (snapshot, mmr): (_, SkewMmr) = self.instance.sync().await?;
        let op = OperationBuilder::new(&owner.encryption_key)
            .spend(snapshot.unspent(owner)[0].clone())
            .output_note(intent.offer)
            .build(rng)?;
        let proven = op.prove_with_mmr(&mmr, &self.artifacts, rng)?;
        send(
            &self.instance.provider,
            proven.transaction(*self.instance.tint.address()),
        )
        .await?;

        self.relay.post(&intent).await?;
        Ok(intent)
    }

    fn maker(&self, account: Account, prices: HashMap<AssetId, U256>) -> Maker {
        Maker::new(
            account,
            prices,
            *self.instance.tint.address(),
            self.instance.provider.clone(),
            self.relay.clone(),
            self.policy.clone(),
            self.artifacts.clone(),
        )
    }
}

/// Asserts `owner` received `intent`'s wanted note, and syncing found nothing else.
fn assert_received(snapshot: &Snapshot, intent: &Intent, owner: &Account) {
    let received = intent.received(snapshot, owner);
    assert_eq!(received.map(|n| n.note), Some(intent.want));
    assert_unspent(snapshot, owner, &[]);
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

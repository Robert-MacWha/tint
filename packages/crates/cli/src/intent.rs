use alloy::primitives::{Address, B256};
use anyhow::Context;
use rand_core::{OsRng, RngCore};
use reqwest::Url;
use tint::{
    account::Account, circuit::join_split::N_INPUTS, note::asset::AssetId,
    operation::OperationBuilder, policy::Policy,
};
use tint_intent_policy::{
    circuit::IntentPolicyCircuit,
    intent::{Intent, OperationBuilderIntentExt},
    policy::IntentPolicy,
};
use tint_intent_relay::{client::RelayClient, server::IntentFilter};
use tracing::{info, warn};

use crate::{
    account::Authorizer,
    chain::{self, Chain},
    config::{self, StoredIntent},
};

/// Intent relay and policy options.
#[derive(clap::Args)]
pub struct IntentArgs {
    #[arg(
        long,
        env = "INTENT_RELAY_URL",
        default_value = "http://127.0.0.1:8080"
    )]
    relay_url: Url,
    #[arg(long, env = "INTENT_POLICY_ADDRESS")]
    intent_policy_address: Option<Address>,
}

impl IntentArgs {
    fn relay(&self) -> RelayClient {
        RelayClient::new(self.relay_url.clone())
    }

    fn policy(&self) -> anyhow::Result<IntentPolicy> {
        let address = self
            .intent_policy_address
            .context("the intent policy address is required (set INTENT_POLICY_ADDRESS)")?;
        let artifacts = config::load_circuit::<IntentPolicyCircuit>("intent")?;
        Ok(IntentPolicy::new(address, artifacts))
    }
}

/// Funds a one-time intent note offering `offer` for `want`, and posts it.
pub async fn post(
    chain: &mut Chain,
    name: &str,
    args: &IntentArgs,
    offer: (Address, u128),
    want: (Address, u128),
    expires_in: u64,
) -> anyhow::Result<()> {
    let stored = config::load_account(name)?;
    let account = stored.account();
    let policy = args.policy()?;

    //? Receive into a fresh account so intents aren't linked to each other or `name`.
    let mut receiver_seed = B256::ZERO;
    OsRng.fill_bytes(receiver_seed.as_mut_slice());
    let receiver = Account::from_seed(&receiver_seed.0, Policy::NONE);

    let offer = (AssetId::from(offer.0), offer.1);
    let expiry = chain.now().await? + expires_in;
    let (intent, cancel_key) = Intent::new(
        policy.address(),
        offer,
        (AssetId::from(want.0), want.1),
        &receiver.pub_account(),
        expiry,
        &mut OsRng,
    )?;

    let op = chain::spend_builder(chain, &account, &[offer])?
        .output_note(intent.offer)
        .change_to(account.pub_account())
        .build(&mut OsRng)?;
    let authorizer = Authorizer::load(&stored, chain.id).await?;
    let proven = chain::prove(chain, &authorizer, op).await?;

    //? Saved before funding, so the cancel key is never lost.
    let mut intents = config::load_intents(name)?;
    intents.push(StoredIntent {
        intent: intent.clone(),
        cancel_key: tint::fr::fr_to_b256(cancel_key.0),
        receiver_seed,
    });
    config::save_intents(name, &intents)?;

    info!("Funding the intent note...");
    chain.send(proven.transaction(chain.tint)).await?;
    let id = args.relay().post(&intent).await?;
    chain.sync().await?;
    println!("{id}");
    Ok(())
}

/// Prints the relay's open intents.
pub async fn list(
    args: &IntentArgs,
    offer_asset: Option<Address>,
    want_asset: Option<Address>,
) -> anyhow::Result<()> {
    let filter = IntentFilter {
        offer_asset,
        want_asset,
    };
    for intent in args.relay().list(&filter).await? {
        println!(
            "{} offers {} {} for {} {}, expires {}",
            intent.id(),
            intent.offer.amount,
            intent.offer.asset,
            intent.want.amount,
            intent.want.asset,
            intent.expiry
        );
    }
    Ok(())
}

/// Prints the state of each intent `name` posted.
pub async fn status(chain: &Chain, name: &str) -> anyhow::Result<()> {
    let now = chain.now().await?;
    for stored in config::load_intents(name)? {
        let intent = &stored.intent;
        let status = intent_status(chain, &stored, now);
        println!(
            "{} offers {} {} for {} {}: {status}",
            intent.id(),
            intent.offer.amount,
            intent.offer.asset,
            intent.want.amount,
            intent.want.asset,
        );
    }
    Ok(())
}

/// Moves a filled intent's received note into `name`'s account.
pub async fn claim(
    chain: &mut Chain,
    name: &str,
    args: &IntentArgs,
    id: B256,
) -> anyhow::Result<()> {
    let account = config::load_account(name)?.account();
    let (index, stored) = find_intent(name, id)?;
    let receiver = stored.receiver();
    let note = stored
        .intent
        .received(&chain.snapshot, &receiver)
        .context("intent hasn't been filled yet")?;
    anyhow::ensure!(!chain.is_spent(&note), "intent was already claimed");

    let op = OperationBuilder::new(&receiver.encryption_key)
        .spend(note.clone())
        .output(account.pub_account(), note.note.asset, note.note.amount)
        .build(&mut OsRng)?;
    let proven = chain::prove_authorized(chain, op)?;
    chain.send(proven.transaction(chain.tint)).await?;
    remove_from_relay(args, &stored).await;
    remove_intent(name, index)?;
    chain.sync().await
}

/// Spends an unfilled intent's note back to `name`, and removes it from the relay.
pub async fn cancel(
    chain: &mut Chain,
    name: &str,
    args: &IntentArgs,
    id: B256,
) -> anyhow::Result<()> {
    let account = config::load_account(name)?.account();
    let (index, stored) = find_intent(name, id)?;
    let intent = &stored.intent;
    anyhow::ensure!(
        !chain.is_spent(&intent.note()),
        "intent was already filled or cancelled"
    );

    let mut op = OperationBuilder::new(&account.encryption_key)
        .spend(intent.note())
        .change_to(account.pub_account())
        .build(&mut OsRng)?;
    let artifacts = config::load_circuit::<IntentPolicyCircuit>("intent")?;
    IntentPolicy::new(intent.policy_address, artifacts).authorize_cancel(
        &mut op,
        intent,
        &stored.cancel_key(),
    )?;
    let proven = chain::prove_authorized(chain, op)?;
    chain.send(proven.transaction(chain.tint)).await?;
    remove_from_relay(args, &stored).await;
    remove_intent(name, index)?;
    chain.sync().await
}

/// Fills a relay intent from `name`'s notes, taking its offer as change.
pub async fn fill(
    chain: &mut Chain,
    name: &str,
    args: &IntentArgs,
    id: B256,
) -> anyhow::Result<()> {
    let stored = config::load_account(name)?;
    let account = stored.account();
    let policy = args.policy()?;
    let intent = args
        .relay()
        .list(&IntentFilter::default())
        .await?
        .into_iter()
        .find(|intent| intent.id() == id)
        .context("no open intent with this id on the relay")?;
    anyhow::ensure!(
        intent.policy_address == policy.address(),
        "intent is bound to an unknown policy {}",
        intent.policy_address
    );
    intent.validate()?;
    anyhow::ensure!(
        !chain.is_spent(&intent.note()),
        "intent was already filled or cancelled"
    );

    let want = (intent.want.asset, intent.want.amount);
    let notes = chain::select_notes(chain, &account, &[want], N_INPUTS - 1)?;
    let mut op = notes
        .into_iter()
        .fold(
            OperationBuilder::new(&account.encryption_key).fill(&intent),
            OperationBuilder::spend,
        )
        .change_to(account.pub_account())
        .build(&mut OsRng)?;
    policy.authorize_fill(&mut op, &intent)?;
    let authorizer = Authorizer::load(&stored, chain.id).await?;
    let proven = chain::prove(chain, &authorizer, op).await?;
    chain.send(proven.transaction(chain.tint)).await?;
    chain.sync().await
}

fn intent_status(chain: &Chain, stored: &StoredIntent, now: u64) -> &'static str {
    let intent = &stored.intent;
    if intent
        .received(&chain.snapshot, &stored.receiver())
        .is_some()
    {
        return "filled, ready to claim";
    }
    if chain.is_spent(&intent.note()) {
        return "spent";
    }
    if chain.mmr.prove_element(intent.offer.hash()).is_none() {
        return "unfunded";
    }
    if intent.expiry < now {
        return "expired, cancel to reclaim";
    }
    "open"
}

/// Removes a settled intent from the relay, which keeps listing it otherwise.
async fn remove_from_relay(args: &IntentArgs, stored: &StoredIntent) {
    let id = stored.intent.id();
    if let Err(err) = args.relay().cancel(id, &stored.cancel_key()).await {
        warn!("Couldn't remove the intent from the relay: {err}");
    }
}

fn find_intent(name: &str, id: B256) -> anyhow::Result<(usize, StoredIntent)> {
    config::load_intents(name)?
        .into_iter()
        .enumerate()
        .find(|(_, stored)| stored.intent.id() == id)
        .with_context(|| format!("\"{name}\" has no intent {id}"))
}

fn remove_intent(name: &str, index: usize) -> anyhow::Result<()> {
    let mut intents = config::load_intents(name)?;
    intents.remove(index);
    config::save_intents(name, &intents)
}

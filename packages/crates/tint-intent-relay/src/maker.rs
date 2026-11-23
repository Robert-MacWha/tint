use std::collections::{HashMap, HashSet};

use alloy_primitives::{Address, B256, U256};
use alloy_provider::{DynProvider, Provider};
use ark_bn254::{Bn254, Fr};
use rand_core::{CryptoRng, RngCore};
use tint::{
    account::Account,
    circuit::join_split::N_INPUTS,
    note::{asset::AssetId, owned::OwnedNote},
    operation::{OperationBuilder, OperationError},
    skew_mmr::{AppendEventsError, SkewMmr, SkewMmrExt},
    syncer::{Syncer, SyncerError, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;
use tint_intent_policy::{
    intent::{Intent, OperationBuilderIntentExt},
    policy::{IntentPolicy, IntentPolicyError},
};
use tracing::info;

use crate::{client::RelayClient, now, server::IntentFilter};

/// A reference maker that settles relay intents.
///
/// Each poll it matches pairs of crossing intents against each other, keeping
/// the surplus, and fills the rest from its own notes when they pay at least
/// its prices. Its account's notes must be under `Policy::NONE`. Proving
/// blocks the calling task.
pub struct Maker {
    account: Account,
    prices: HashMap<AssetId, U256>,
    tint: Address,
    provider: DynProvider,
    relay: RelayClient,
    policy: IntentPolicy,
    artifacts: Artifacts<Bn254>,
}

/// The intents one operation settles, and the maker's notes it spends.
struct Fill<'a> {
    intents: Vec<&'a Intent>,
    notes: Vec<OwnedNote>,
}

/// The note set as of the last sync.
struct Chain {
    mmr: SkewMmr,
    nullifiers: HashSet<Fr>,
    notes: Vec<OwnedNote>,
}

#[derive(Debug, thiserror::Error)]
pub enum MakerError {
    #[error("relay error: {0}")]
    Relay(#[from] reqwest::Error),
    #[error("sync error: {0}")]
    Sync(#[from] SyncerError),
    #[error("note set error: {0}")]
    Mmr(#[from] AppendEventsError),
    #[error("operation error: {0}")]
    Operation(#[from] OperationError),
    #[error("intent policy error: {0}")]
    Policy(#[from] IntentPolicyError),
    #[error("transaction error: {0}")]
    Transaction(#[from] alloy_provider::PendingTransactionError),
    #[error("rpc error: {0}")]
    Rpc(#[from] alloy_provider::transport::TransportError),
    #[error("settlement {0} reverted")]
    Reverted(B256),
}

impl Maker {
    /// Creates a maker valuing each asset at `prices`, in a common unit per base unit.
    #[must_use]
    pub fn new(
        account: Account,
        prices: HashMap<AssetId, U256>,
        tint: Address,
        provider: DynProvider,
        relay: RelayClient,
        policy: IntentPolicy,
        artifacts: Artifacts<Bn254>,
    ) -> Self {
        Self {
            account,
            prices,
            tint,
            provider,
            relay,
            policy,
            artifacts,
        }
    }

    /// Settles every open intent it can, returning the ids settled.
    ///
    /// # Errors
    ///
    /// Errors if the relay or chain can't be read, or a settlement fails.
    pub async fn poll<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Result<Vec<B256>, MakerError> {
        let intents = self.relay.list(&IntentFilter::default()).await?;
        let mut chain = self.sync().await?;
        let mut settled = Vec::new();

        for (i, intent) in intents.iter().enumerate() {
            let open =
                |other: &Intent| !settled.contains(&other.id()) && self.is_open(other, &chain);
            let Some(fill) = self.plan(intent, &intents[i + 1..], open, &chain) else {
                continue;
            };

            self.settle(&fill, &chain.mmr, rng).await?;
            settled.extend(fill.intents.iter().map(|i| i.id()));
            chain = self.sync().await?;
        }
        Ok(settled)
    }

    /// Picks how to settle `intent`: against a crossing intent in `rest`, or
    /// from the maker's notes.
    fn plan<'a>(
        &self,
        intent: &'a Intent,
        rest: &'a [Intent],
        open: impl Fn(&Intent) -> bool,
        chain: &Chain,
    ) -> Option<Fill<'a>> {
        if !open(intent) {
            return None;
        }

        if let Some(other) = rest.iter().find(|o| crosses(intent, o) && open(o)) {
            return Some(Fill {
                intents: vec![intent, other],
                notes: Vec::new(),
            });
        }

        if !self.is_profitable(intent) {
            return None;
        }
        let notes = select_notes(&chain.notes, intent.want.asset, intent.want.amount)?;
        Some(Fill {
            intents: vec![intent],
            notes,
        })
    }

    /// Builds, proves, and submits the operation settling `fill`.
    async fn settle<R: RngCore + CryptoRng>(
        &self,
        fill: &Fill<'_>,
        mmr: &SkewMmr,
        rng: &mut R,
    ) -> Result<(), MakerError> {
        let mut builder = OperationBuilder::new(&self.account.encryption_key)
            .change_to(self.account.pub_account());
        for intent in &fill.intents {
            builder = builder.fill(intent);
        }
        for note in &fill.notes {
            builder = builder.spend(note.clone());
        }

        let mut op = builder.build(rng)?;
        for intent in &fill.intents {
            self.policy.authorize_fill(&mut op, intent)?;
        }
        let proven = op.prove_with_mmr(mmr, &self.artifacts, rng)?;

        let receipt = self
            .provider
            .send_transaction(proven.transaction(self.tint))
            .await?
            .get_receipt()
            .await?;
        if !receipt.status() {
            return Err(MakerError::Reverted(receipt.transaction_hash));
        }
        info!(intents = fill.intents.len(), tx = ?receipt.transaction_hash, "Settled intents");
        Ok(())
    }

    /// Whether `intent` can be filled: valid, unexpired, unspent, and bound to
    /// this maker's policy deployment.
    fn is_open(&self, intent: &Intent, chain: &Chain) -> bool {
        intent.policy_address == self.policy.address()
            && intent.validate().is_ok()
            && intent.expiry >= now()
            && !chain.nullifiers.contains(&intent.note().nullifier())
            && chain.mmr.prove_element(intent.offer.hash()).is_some()
    }

    /// Whether `intent`'s offer is worth at least what it wants, at the maker's prices.
    fn is_profitable(&self, intent: &Intent) -> bool {
        let (Some(offer_price), Some(want_price)) = (
            self.prices.get(&intent.offer.asset),
            self.prices.get(&intent.want.asset),
        ) else {
            return false;
        };
        let offer = U256::from(intent.offer.amount).saturating_mul(*offer_price);
        let want = U256::from(intent.want.amount).saturating_mul(*want_price);
        offer >= want
    }

    async fn sync(&self) -> Result<Chain, MakerError> {
        let snapshot = RpcSyncer::new(self.provider.clone())
            .sync(self.tint, ..)
            .await?;
        let mut mmr = SkewMmr::new();
        mmr.append_events(&snapshot.events)?;
        Ok(Chain {
            mmr,
            nullifiers: snapshot.nullifiers(),
            notes: snapshot.unspent(&self.account),
        })
    }
}

/// Whether `a` and `b` each offer at least what the other wants.
fn crosses(a: &Intent, b: &Intent) -> bool {
    a.offer.asset == b.want.asset
        && b.offer.asset == a.want.asset
        && a.offer.amount >= b.want.amount
        && b.offer.amount >= a.want.amount
}

/// Picks notes of `asset` totalling at least `amount`, leaving an input slot
/// for the intent.
fn select_notes(notes: &[OwnedNote], asset: AssetId, amount: u128) -> Option<Vec<OwnedNote>> {
    let mut selected = Vec::new();
    let mut total = 0u128;
    for note in notes.iter().filter(|n| n.note.asset == asset) {
        if total >= amount || selected.len() == N_INPUTS - 1 {
            break;
        }
        total = total.saturating_add(note.note.amount);
        selected.push(note.clone());
    }
    (total >= amount).then_some(selected)
}

use alloy::{
    network::TransactionBuilder,
    primitives::{Address, B256, hex},
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::Context;
use ark_bn254::Bn254;
use rand_core::OsRng;
use tint::{
    account::{Account, pub_account::PubAccount},
    circuit::join_split::N_INPUTS,
    deposit::Deposit,
    note::{asset::AssetId, owned::OwnedNote},
    operation::OperationBuilder,
    provider_ext::TintProviderExt,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Snapshot, Syncer, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;
use tint_web_protocol::{Connection, Reply, RequestKind, Sponsor};

use crate::{
    account_state::account_state,
    artifacts::fetch_artifacts,
    keys::{account_root, derive_account, recover_accounts},
    progress, public, revert, sponsor, store,
};

/// Handles requests, caching the proving artifacts and synced chain state
/// between them.
#[derive(Default)]
pub struct Handler {
    artifacts: Option<Artifacts<Bn254>>,
    chain: Option<ChainState>,
}

/// A signing connection to one Tint deployment, acting for one of the
/// signer's accounts.
pub struct Session {
    pub provider: DynProvider,
    rpc_url: String,
    pub tint: Address,
    from: Address,
    /// The root the signer's accounts are derived from.
    root: B256,
    pub account: Account,
}

/// Tint's synced events and skew-MMR for one deployment.
struct ChainState {
    rpc_url: String,
    snapshot: Snapshot,
    mmr: SkewMmr,
}

impl Handler {
    pub async fn handle(&mut self, id: u64, kind: RequestKind) -> anyhow::Result<Reply> {
        match kind {
            RequestKind::Accounts { conn, min } => {
                let session = Session::connect(&conn, 0)?;
                self.sync(id, &session).await?;
                let chain = self.chain.as_ref().context("not synced")?;
                Ok(Reply::Accounts(recover_accounts(
                    &chain.snapshot,
                    session.root,
                    min,
                )))
            }
            RequestKind::Sync { conn, account } => {
                let session = Session::connect(&conn, account)?;
                self.sync(id, &session).await?;
                self.state(&session)
            }
            RequestKind::Public { conn, tokens } => {
                let session = Session::connect(&conn, 0)?;
                let state = public::state(&session.provider, session.from, &tokens).await?;
                Ok(Reply::Public(state))
            }
            RequestKind::Faucet { conn, eth, tokens } => {
                let session = Session::connect(&conn, 0)?;
                progress(id, "Funding");
                public::fund(&session.provider, session.from, eth, &tokens).await?;
                let tokens: Vec<_> = tokens.iter().map(|t| t.token).collect();
                let state = public::state(&session.provider, session.from, &tokens).await?;
                Ok(Reply::Public(state))
            }
            RequestKind::Deposit {
                conn,
                account,
                token,
                amount,
            } => {
                let session = Session::connect(&conn, account)?;
                self.deposit(id, &session, token, amount).await
            }
            RequestKind::Transfer {
                conn,
                account,
                to,
                token,
                amount,
                sponsor,
            } => {
                let session = Session::connect(&conn, account)?;
                let bytes = hex::decode(to.trim()).context("invalid receive address")?;
                let recipient =
                    PubAccount::from_bytes(&bytes).context("invalid receive address")?;
                self.operate(id, &session, token, amount, sponsor, |op, asset| {
                    op.output(recipient, asset, amount)
                })
                .await
            }
            RequestKind::Withdraw {
                conn,
                account,
                to,
                token,
                amount,
                sponsor,
            } => {
                let session = Session::connect(&conn, account)?;
                self.operate(id, &session, token, amount, sponsor, |op, asset| {
                    op.withdraw(to, asset, amount)
                })
                .await
            }
        }
    }

    /// Approves and deposits `amount` of `token` into `session`'s account.
    async fn deposit(
        &mut self,
        id: u64,
        session: &Session,
        token: Address,
        amount: u128,
    ) -> anyhow::Result<Reply> {
        let deposit = Deposit::new(
            session.tint,
            session.account.pub_account(),
            AssetId::from(token),
            amount,
            &mut OsRng,
        );

        progress(id, "Approving token");
        session.send(deposit.approval()).await?;
        progress(id, "Depositing");
        session.send(deposit.transaction(&mut OsRng)?).await?;

        self.sync(id, session).await?;
        self.state(session)
    }

    /// Proves and sends an operation spending notes covering `amount` of
    /// `token`, with `add`'s outputs or withdrawals, sending the change back
    /// to `session`'s account. With a `sponsor`, the notes also pay the gas.
    async fn operate(
        &mut self,
        id: u64,
        session: &Session,
        token: Address,
        amount: u128,
        sponsor: Option<Sponsor>,
        add: impl FnOnce(OperationBuilder, AssetId) -> OperationBuilder,
    ) -> anyhow::Result<Reply> {
        self.sync(id, session).await?;
        if self.artifacts.is_none() {
            self.artifacts = Some(fetch_artifacts(id).await?);
        }
        let (Some(chain), Some(artifacts)) = (&self.chain, &self.artifacts) else {
            anyhow::bail!("worker lost its synced state");
        };

        let asset = AssetId::from(token);
        let notes = pick_notes(&chain.snapshot, &session.account, asset, amount)?;
        let builder = notes
            .into_iter()
            .fold(OperationBuilder::new(), OperationBuilder::spend)
            .change_to(session.account.pub_account());
        let builder = add(builder, asset);

        if let Some(sponsor) = sponsor {
            sponsor::send(id, &sponsor, session, builder, &chain.mmr, artifacts).await?;
        } else {
            let op = builder.build(&mut OsRng)?;
            progress(id, "Proving");
            let proven = op.prove_with_mmr(&chain.mmr, artifacts, &mut OsRng)?;
            progress(id, "Sending operation");
            session.send(proven.transaction(session.tint)).await?;
        }

        self.sync(id, session).await?;
        self.state(session)
    }

    /// Syncs the chain state with `session`'s deployment, discarding it and
    /// its stored copy on failure so the next sync starts clean.
    async fn sync(&mut self, id: u64, session: &Session) -> anyhow::Result<()> {
        progress(id, "Syncing");
        let result = self.try_sync(id, session).await;
        if result.is_err() {
            self.chain = None;
            if let Err(e) = store::delete(&session.rpc_url, session.tint).await {
                progress(id, &format!("Couldn't delete stored events: {e:#}"));
            }
        }
        result
    }

    /// Fetches events since the last sync, overlapping its final block,
    /// checks the resulting MMR against the chain, and stores the result.
    async fn try_sync(&mut self, id: u64, session: &Session) -> anyhow::Result<()> {
        // Calls to an address without code return no data, which otherwise
        // surfaces as an opaque ABI decoding error.
        if session.provider.get_code_at(session.tint).await?.is_empty() {
            anyhow::bail!(
                "no contract at Tint address {} on {}",
                session.tint,
                session.rpc_url
            );
        }

        let chain = self.chain_for(id, session).await;

        // Tint has no known deployment block to start from, so fetch everything
        // in a single `eth_getLogs` call rather than batching from genesis.
        let syncer = RpcSyncer::new(session.provider.clone()).with_batch_size(u64::MAX);
        let from = chain.snapshot.range.end.saturating_sub(1);
        let next = syncer.sync(session.tint, from..).await?;

        chain.mmr.append_events(&next.events)?;
        chain.snapshot.extend(next)?;

        // The contract pre-fills its frontier history to avoid cold writes, so an
        // empty frontier never verifies. An empty MMR can't back any proofs anyway.
        if chain.mmr.is_empty() {
            return Ok(());
        }
        let verified = session
            .provider
            .tint_verify_mmr(session.tint, &chain.mmr)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        if !verified {
            anyhow::bail!("local skew-MMR does not match the on-chain frontier");
        }

        if let Err(e) = store::save(&session.rpc_url, &chain.snapshot).await {
            progress(id, &format!("Couldn't store synced events: {e:#}"));
        }
        Ok(())
    }

    /// The cached chain state for `session`'s deployment, replacing any cached
    /// for another with the one stored for it, if any.
    async fn chain_for(&mut self, id: u64, session: &Session) -> &mut ChainState {
        let cached = self
            .chain
            .as_ref()
            .is_some_and(|c| c.rpc_url == session.rpc_url && c.snapshot.tint == session.tint);
        if !cached {
            self.chain = Some(load_chain(id, session).await);
        }
        self.chain.get_or_insert_with(|| ChainState::empty(session))
    }

    fn state(&self, session: &Session) -> anyhow::Result<Reply> {
        let chain = self.chain.as_ref().context("not synced")?;
        Ok(Reply::State(account_state(
            &chain.snapshot,
            &chain.mmr,
            &session.account,
            session.from,
        )))
    }
}

impl ChainState {
    fn empty(session: &Session) -> Self {
        Self {
            rpc_url: session.rpc_url.clone(),
            snapshot: Snapshot::new(session.tint, 0..0, Vec::new()),
            mmr: SkewMmr::new(),
        }
    }

    /// Rebuilds the skew-MMR from `snapshot`'s events.
    fn from_snapshot(session: &Session, snapshot: Snapshot) -> anyhow::Result<Self> {
        let mut mmr = SkewMmr::new();
        mmr.append_events(&snapshot.events)?;
        Ok(Self {
            rpc_url: session.rpc_url.clone(),
            snapshot,
            mmr,
        })
    }
}

impl Session {
    /// Connects as the signer's account at `index`.
    fn connect(conn: &Connection, index: u32) -> anyhow::Result<Self> {
        let signer = PrivateKeySigner::from_bytes(&conn.private_key)?;
        let from = signer.address();
        let root = account_root(&signer)?;
        let provider = ProviderBuilder::new()
            .wallet(signer)
            .connect_http(conn.rpc_url.parse().context("invalid RPC URL")?)
            .erased();

        Ok(Self {
            provider,
            rpc_url: conn.rpc_url.clone(),
            tint: conn.tint,
            from,
            root,
            account: derive_account(root, index),
        })
    }

    /// Sends `tx` and waits for it to succeed.
    async fn send(&self, tx: TransactionRequest) -> anyhow::Result<()> {
        let receipt = self
            .provider
            .send_transaction(tx.with_from(self.from))
            .await
            .map_err(revert::describe)?
            .get_receipt()
            .await?;
        if !receipt.status() {
            anyhow::bail!("transaction {} reverted", receipt.transaction_hash);
        }
        Ok(())
    }
}

/// The chain state stored for `session`'s deployment, or an empty one if
/// none is stored or it can't be loaded.
async fn load_chain(id: u64, session: &Session) -> ChainState {
    let stored = store::load(&session.rpc_url, session.tint).await;
    let chain = stored.and_then(|s| s.map(|s| ChainState::from_snapshot(session, s)).transpose());
    match chain {
        Ok(Some(chain)) => {
            let events = chain.snapshot.events.len();
            progress(id, &format!("Loaded {events} stored events"));
            chain
        }
        Ok(None) => ChainState::empty(session),
        Err(e) => {
            progress(id, &format!("Couldn't load stored events: {e:#}"));
            ChainState::empty(session)
        }
    }
}

/// Picks as many of the account's spendable `asset` notes as an operation
/// can spend, largest first, so spending them also consolidates the
/// account's notes. Every note is nullified once eventually, so spending more
/// now costs no extra gas overall.
fn pick_notes(
    snapshot: &Snapshot,
    account: &Account,
    asset: AssetId,
    amount: u128,
) -> anyhow::Result<Vec<OwnedNote>> {
    let mut notes: Vec<_> = snapshot
        .unspent(account)
        .into_iter()
        .filter(|note| note.note.asset == asset)
        .collect();
    notes.sort_by_key(|note| std::cmp::Reverse(note.note.amount));
    notes.truncate(N_INPUTS);

    let total: u128 = notes.iter().map(|note| note.note.amount).sum();
    if total < amount {
        anyhow::bail!(
            "an operation can spend up to {N_INPUTS} notes, and the largest of this token only hold {total}"
        );
    }
    Ok(notes)
}

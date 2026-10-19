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
    progress, revert, sponsor,
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

    /// Proves and sends an operation spending a note covering `amount` of
    /// `token`, with `add`'s outputs or withdrawals, sending the change back
    /// to `session`'s account. With a `sponsor`, the note also pays the gas.
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
        let builder = OperationBuilder::new()
            .spend(pick_note(&chain.snapshot, &session.account, asset, amount)?)
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

    /// Syncs the chain state with `session`'s deployment, discarding it on
    /// failure so the next sync starts clean.
    async fn sync(&mut self, id: u64, session: &Session) -> anyhow::Result<()> {
        progress(id, "Syncing");
        let result = self.try_sync(session).await;
        if result.is_err() {
            self.chain = None;
        }
        result
    }

    /// Fetches events since the last sync, overlapping its final block, and
    /// checks the resulting MMR against the chain.
    async fn try_sync(&mut self, session: &Session) -> anyhow::Result<()> {
        // Calls to an address without code return no data, which otherwise
        // surfaces as an opaque ABI decoding error.
        if session.provider.get_code_at(session.tint).await?.is_empty() {
            anyhow::bail!(
                "no contract at Tint address {} on {}",
                session.tint,
                session.rpc_url
            );
        }

        let chain = self.chain_for(session);

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
        Ok(())
    }

    /// The cached chain state for `session`'s deployment, replacing any cached
    /// for another.
    fn chain_for(&mut self, session: &Session) -> &mut ChainState {
        let fresh = || ChainState {
            rpc_url: session.rpc_url.clone(),
            snapshot: Snapshot::new(session.tint, 0..0, Vec::new()),
            mmr: SkewMmr::new(),
        };
        if !self
            .chain
            .as_ref()
            .is_some_and(|c| c.rpc_url == session.rpc_url && c.snapshot.tint == session.tint)
        {
            self.chain = Some(fresh());
        }
        self.chain.get_or_insert_with(fresh)
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

/// Picks a single spendable note covering `amount` of `asset`. Tint supports
/// spending multiple notes at once, but this keeps things simple and only
/// ever spends one.
fn pick_note(
    snapshot: &Snapshot,
    account: &Account,
    asset: AssetId,
    amount: u128,
) -> anyhow::Result<OwnedNote> {
    snapshot
        .unspent(account)
        .into_iter()
        .find(|note| note.note.asset == asset && note.note.amount >= amount)
        .context("no single spendable note covers that amount of this token")
}

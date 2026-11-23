use std::collections::BTreeMap;

use alloy::{
    network::TransactionBuilder,
    primitives::{Address, B256, U256},
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::{TransactionReceipt, TransactionRequest},
};
use alloy_provider::ext::TenderlyAdminApi;
use alloy_signer_local::PrivateKeySigner;
use anyhow::Context;
use rand_core::OsRng;
use tint::{
    account::{Account, pub_account::PubAccount},
    circuit::join_split::{JoinSplitCircuit, N_INPUTS},
    deposit::Deposit,
    note::{asset::AssetId, owned::OwnedNote},
    operation::{Operation, OperationBuilder, ProvenOperation},
    provider_ext::TintProviderExt,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Snapshot, Syncer, rpc::RpcSyncer},
};
use tracing::{info, warn};

use crate::{
    account::Authorizer,
    config::{self, StoredAccount},
    paymaster::{Sponsor, SponsorArgs},
};

/// Chain connection options shared by on-chain commands.
#[derive(clap::Args)]
pub struct ChainArgs {
    #[arg(long, env = "RPC_URL")]
    rpc_url: String,
    /// Key of the EOA that sends transactions and pays their gas
    #[arg(long, env = "PRIVATE_KEY")]
    private_key: Option<B256>,
    #[arg(long, env = "TINT_ADDRESS")]
    tint_address: Address,
}

/// A provider plus a synced view of a Tint deployment's note set.
pub struct Chain {
    pub provider: DynProvider,
    pub tint: Address,
    pub id: u64,
    pub snapshot: Snapshot,
    pub mmr: SkewMmr,
    from: Option<Address>,
}

impl Chain {
    /// Connects to `args`'s chain and syncs, resuming from the saved snapshot.
    pub async fn connect(args: &ChainArgs) -> anyhow::Result<Self> {
        let url = args.rpc_url.parse().context("invalid RPC URL")?;
        let signer = args
            .private_key
            .map(|key| PrivateKeySigner::from_bytes(&key))
            .transpose()?;
        let from = signer.as_ref().map(PrivateKeySigner::address);
        let provider = match signer {
            Some(signer) => ProviderBuilder::new()
                .wallet(signer)
                .connect_http(url)
                .erased(),
            None => ProviderBuilder::new().connect_http(url).erased(),
        };
        let chain_id = provider.get_chain_id().await?;

        let saved = config::load_snapshot(chain_id, args.tint_address)?;
        let resumed = saved.is_some();
        let mut chain = Self {
            provider,
            tint: args.tint_address,
            id: chain_id,
            snapshot: saved.unwrap_or_else(|| empty_snapshot(args.tint_address)),
            mmr: SkewMmr::new(),
            from,
        };
        chain.mmr.append_events(&chain.snapshot.events)?;

        info!("Syncing from block {}...", chain.snapshot.range.end);
        let Err(err) = chain.sync().await else {
            return Ok(chain);
        };
        if !resumed {
            return Err(err);
        }

        warn!("Saved snapshot is stale ({err}), resyncing from scratch...");
        chain.snapshot = empty_snapshot(chain.tint);
        chain.mmr = SkewMmr::new();
        chain.sync().await?;
        Ok(chain)
    }

    /// Fetches events since the last sync, checks the MMR against the chain,
    /// and saves the snapshot.
    pub async fn sync(&mut self) -> anyhow::Result<()> {
        // Tint has no known deployment block to start from, so fetch everything
        // in a single `eth_getLogs` call rather than batching from genesis.
        let syncer = RpcSyncer::new(self.provider.clone()).with_batch_size(u64::MAX);
        let from = self.snapshot.range.end.saturating_sub(1);
        let next = syncer.sync(self.tint, from..).await?;

        self.mmr.append_events(&next.events)?;
        self.snapshot.extend(next)?;

        // The contract pre-fills its frontier history to avoid cold writes, so an
        // empty frontier never verifies. An empty MMR can't back any proofs anyway.
        if !self.mmr.is_empty() {
            let verified = self
                .provider
                .tint_verify_mmr(self.tint, &self.mmr)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            anyhow::ensure!(
                verified,
                "local skew-MMR does not match the on-chain frontier"
            );
        }
        config::save_snapshot(self.id, &self.snapshot)
    }

    /// Sends `tx` from the `PRIVATE_KEY` EOA, failing if it reverts.
    pub async fn send(&self, tx: TransactionRequest) -> anyhow::Result<TransactionReceipt> {
        let from = self
            .from
            .context("sending a transaction needs PRIVATE_KEY (or --sponsor)")?;
        let receipt = self
            .provider
            .send_transaction(tx.with_from(from))
            .await?
            .get_receipt()
            .await?;
        anyhow::ensure!(receipt.status(), "tx {} reverted", receipt.transaction_hash);
        info!(
            "tx {} confirmed (gas used: {})",
            receipt.transaction_hash, receipt.gas_used
        );
        Ok(receipt)
    }

    /// The latest block's timestamp.
    pub async fn now(&self) -> anyhow::Result<u64> {
        let block = self
            .provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
            .await?
            .context("no latest block")?;
        Ok(block.header.timestamp)
    }

    /// Whether `note` has been spent, as of the last sync.
    pub fn is_spent(&self, note: &OwnedNote) -> bool {
        self.snapshot.nullifiers().contains(&note.nullifier())
    }
}

/// Sums `account`'s unspent notes per asset.
pub fn balances(chain: &Chain, account: &Account) -> BTreeMap<Address, u128> {
    let mut balances = BTreeMap::new();
    for note in chain.snapshot.unspent(account) {
        *balances.entry(note.note.asset.into()).or_default() += note.note.amount;
    }
    balances
}

/// Auto-approves `token` for `amount` and deposits it into Tint for `account`.
pub async fn shield(
    chain: &mut Chain,
    receiver: PubAccount,
    token: Address,
    amount: u128,
) -> anyhow::Result<()> {
    let deposit = Deposit::new(
        chain.tint,
        receiver,
        AssetId::from(token),
        amount,
        &mut OsRng,
    );

    info!("Approving Tint to spend {amount} of {token}...");
    chain.send(deposit.approval()).await?;

    info!("Depositing...");
    chain.send(deposit.transaction(&mut OsRng)?).await?;
    chain.sync().await
}

/// Sends `amount` of `token` to `to`, returning change to `from`.
pub async fn transfer(
    chain: &mut Chain,
    from: &StoredAccount,
    to: PubAccount,
    token: Address,
    amount: u128,
    sponsor: &SponsorArgs,
) -> anyhow::Result<()> {
    let asset = AssetId::from(token);
    submit(chain, from, &[(asset, amount)], sponsor, |builder| {
        builder.output(to, asset, amount)
    })
    .await
}

/// Withdraws `amount` of `token` to the plain address `to`, returning change to `from`.
pub async fn unshield(
    chain: &mut Chain,
    from: &StoredAccount,
    to: Address,
    token: Address,
    amount: u128,
    sponsor: &SponsorArgs,
) -> anyhow::Result<()> {
    let asset = AssetId::from(token);
    submit(chain, from, &[(asset, amount)], sponsor, |builder| {
        builder.withdraw(to, asset, amount)
    })
    .await
}

/// Spends `from`'s notes covering `needs` into `outputs`, returning change to
/// `from`, then submits directly or through the paymaster.
async fn submit(
    chain: &mut Chain,
    from: &StoredAccount,
    needs: &[(AssetId, u128)],
    sponsor: &SponsorArgs,
    outputs: impl FnOnce(OperationBuilder) -> OperationBuilder,
) -> anyhow::Result<()> {
    let account = from.account();
    let sponsor = Sponsor::connect(chain, sponsor).await?;
    let mut needs = needs.to_vec();
    if let Some(sponsor) = &sponsor {
        needs.push(sponsor.fee_budget());
    }

    let builder = spend_builder(chain, &account, &needs)?.change_to(account.pub_account());
    let builder = outputs(builder);
    let authorizer = Authorizer::load(from, chain.id).await?;

    if let Some(sponsor) = sponsor {
        Box::pin(sponsor.submit(chain, &account, &authorizer, builder)).await?;
        return chain.sync().await;
    }

    let op = builder.build(&mut OsRng)?;
    let proven = prove(chain, &authorizer, op).await?;
    chain.send(proven.transaction(chain.tint)).await?;
    chain.sync().await
}

/// An [`OperationBuilder`] spending `account`'s notes covering `needs`.
pub fn spend_builder(
    chain: &Chain,
    account: &Account,
    needs: &[(AssetId, u128)],
) -> anyhow::Result<OperationBuilder> {
    let notes = select_notes(chain, account, needs, N_INPUTS)?;
    Ok(notes.into_iter().fold(
        OperationBuilder::new(&account.encryption_key),
        OperationBuilder::spend,
    ))
}

/// Authorizes `op` under `authorizer` and proves it.
pub async fn prove(
    chain: &Chain,
    authorizer: &Authorizer,
    mut op: Operation,
) -> anyhow::Result<ProvenOperation> {
    authorizer.authorize(&mut op, chain.now().await?).await?;
    prove_authorized(chain, op)
}

/// Proves an already-authorized `op`.
pub fn prove_authorized(chain: &Chain, op: Operation) -> anyhow::Result<ProvenOperation> {
    let artifacts = config::load_circuit::<JoinSplitCircuit>("join_split")?;
    info!("Proving operation...");
    Ok(op.prove_with_mmr(&chain.mmr, &artifacts, &mut OsRng)?)
}

/// Picks the largest of `account`'s notes until each need is covered, using
/// at most `max_notes` notes.
pub fn select_notes(
    chain: &Chain,
    account: &Account,
    needs: &[(AssetId, u128)],
    max_notes: usize,
) -> anyhow::Result<Vec<OwnedNote>> {
    let mut totals = BTreeMap::<Address, u128>::new();
    for &(asset, amount) in needs {
        *totals.entry(asset.into()).or_default() += amount;
    }

    let mut unspent = chain.snapshot.unspent(account);
    unspent.sort_by_key(|note| std::cmp::Reverse(note.note.amount));

    let mut selected = Vec::new();
    for (asset, amount) in totals {
        let mut covered = 0u128;
        for note in unspent
            .iter()
            .filter(|n| Address::from(n.note.asset) == asset)
        {
            if covered >= amount {
                break;
            }
            covered += note.note.amount;
            selected.push(note.clone());
        }
        anyhow::ensure!(
            covered >= amount,
            "insufficient {asset}: need {amount}, have {covered}"
        );
    }
    anyhow::ensure!(
        selected.len() <= max_notes,
        "covering this needs {} notes, but an operation spends at most {max_notes}",
        selected.len()
    );
    Ok(selected)
}

/// Derives the Ethereum address for a private key.
pub fn eth_address(private_key: B256) -> anyhow::Result<Address> {
    Ok(PrivateKeySigner::from_bytes(&private_key)?.address())
}

/// Overwrites `address`'s native balance via Tenderly's `tenderly_setBalance`
/// admin cheatcode. Only works against a Tenderly virtual testnet.
pub async fn set_balance(rpc_url: &str, address: Address, amount: U256) -> anyhow::Result<()> {
    let provider = connect_admin(rpc_url)?;
    let tx_hash = provider.tenderly_set_balance(address, amount).await?;
    info!("tx {tx_hash} confirmed");
    Ok(())
}

/// Overwrites `address`'s balance of `token` via Tenderly's
/// `tenderly_setErc20Balance` admin cheatcode. Only works against a Tenderly
/// virtual testnet.
pub async fn set_erc20_balance(
    rpc_url: &str,
    token: Address,
    address: Address,
    amount: U256,
) -> anyhow::Result<()> {
    let provider = connect_admin(rpc_url)?;
    let tx_hash = provider
        .tenderly_set_erc20_balance(token, address, amount)
        .await?;
    info!("tx {tx_hash} confirmed");
    Ok(())
}

fn empty_snapshot(tint: Address) -> Snapshot {
    Snapshot::new(tint, 0..0, Vec::new())
}

/// Builds a bare (unsigned) provider connection for admin RPC calls that
/// don't require a wallet, such as Tenderly's balance-override cheatcodes.
fn connect_admin(rpc_url: &str) -> anyhow::Result<DynProvider> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls crypto provider"))?;

    Ok(ProviderBuilder::new()
        .connect_http(rpc_url.parse().context("invalid RPC URL")?)
        .erased())
}

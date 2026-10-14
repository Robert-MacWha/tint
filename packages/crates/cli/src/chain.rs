use alloy::{
    network::TransactionBuilder,
    primitives::{Address, B256, U256},
    providers::{DynProvider, Provider as AlloyProvider, ProviderBuilder},
    rpc::types::TransactionRequest,
};
use alloy_provider::ext::TenderlyAdminApi;
use alloy_signer_local::PrivateKeySigner;
use anyhow::Context;
use ark_bn254::Bn254;
use rand_core::OsRng;
use tint::{
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    deposit::Deposit,
    note::{asset::AssetId, owned::OwnedNote},
    operation::{OperationBuilder, UnsignedOperation},
    provider_ext::TintProviderExt,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Snapshot, Syncer, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;

use crate::{account::Authorizer, config};

/// A signing provider connection plus the Tint state needed to shield,
/// transfer, and unshield funds for a single local account.
pub struct Session {
    provider: DynProvider,
    tint_address: Address,
    from: Address,
    account: Account,
    authorizer: Authorizer,
    artifacts: Artifacts<Bn254>,
    snapshot: Snapshot,
    mmr: SkewMmr,
}

/// Signs with a local private key and syncs Tint's state for `account`,
/// mirroring the setup in `tint`'s shield/transfer/unshield integration tests.
pub async fn connect(
    account: Account,
    authorizer: Authorizer,
    tint_address: Address,
    rpc_url: &str,
    private_key: B256,
) -> anyhow::Result<Session> {
    let signer = PrivateKeySigner::from_slice(private_key.as_slice())?;
    let from = signer.address();

    tracing::info!("Connecting to {rpc_url}...");
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse().context("invalid RPC URL")?)
        .erased();

    let artifacts = config::load_circuit::<JoinSplitCircuit>("join_split")?;

    let mut session = Session {
        provider,
        tint_address,
        from,
        account,
        authorizer,
        artifacts,
        snapshot: Snapshot::new(tint_address, 0..0, Vec::new()),
        mmr: SkewMmr::new(),
    };

    tracing::info!("Syncing with chain...");
    sync(&mut session).await?;
    Ok(session)
}

/// Derives the Ethereum address for a private key.
pub fn eth_address(private_key: B256) -> anyhow::Result<Address> {
    let signer = PrivateKeySigner::from_slice(private_key.as_slice())?;
    Ok(signer.address())
}

/// Overwrites `address`'s native balance via Tenderly's `tenderly_setBalance`
/// admin cheatcode. Only works against a Tenderly virtual testnet.
pub async fn set_balance(rpc_url: &str, address: Address, amount: U256) -> anyhow::Result<()> {
    let provider = connect_admin(rpc_url)?;
    let tx_hash = provider.tenderly_set_balance(address, amount).await?;
    tracing::info!("tx {tx_hash} confirmed");
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
    tracing::info!("tx {tx_hash} confirmed");
    Ok(())
}

/// Auto-approves `token` for `amount` and deposits it into Tint.
pub async fn shield(session: &mut Session, token: Address, amount: u128) -> anyhow::Result<()> {
    let deposit = Deposit::new(
        session.tint_address,
        session.account.pub_account(),
        AssetId::from(token),
        amount,
        &mut OsRng,
    );

    tracing::info!("Approving Tint to spend {amount} of {token}...");
    send(session, deposit.approval()).await?;

    tracing::info!("Depositing...");
    send(session, deposit.request(&mut OsRng)?).await?;

    sync(session).await?;
    print_balance(session);
    Ok(())
}

/// Transfers `amount` of `token` to `to` (a local account name), sending any
/// leftover change back to `account`.
pub async fn transfer(
    session: &mut Session,
    to: &str,
    token: Address,
    amount: u128,
) -> anyhow::Result<()> {
    let asset = AssetId::from(token);
    let recipient = config::load_pub_account(to)?;

    let op = OperationBuilder::new()
        .spend(pick_note(session, asset, amount)?)
        .output(recipient, asset, amount)
        .change_to(session.account.pub_account())
        .build(&mut OsRng)?;

    tracing::info!("Building transfer proof...");
    operate(session, op).await
}

/// Withdraws `amount` of `token` to the plain address `to`, sending any
/// leftover change back to `account`.
pub async fn unshield(
    session: &mut Session,
    to: Address,
    token: Address,
    amount: u128,
) -> anyhow::Result<()> {
    let asset = AssetId::from(token);

    let op = OperationBuilder::new()
        .spend(pick_note(session, asset, amount)?)
        .withdraw(to, asset, amount)
        .change_to(session.account.pub_account())
        .build(&mut OsRng)?;

    tracing::info!("Building unshield proof...");
    operate(session, op).await
}

/// Prints the shielded balance of each token for the connected account.
pub fn print_balance(session: &Session) {
    let balances = session.snapshot.unspent(&session.account).iter().fold(
        std::collections::HashMap::<AssetId, u128>::new(),
        |mut acc, note| {
            *acc.entry(note.note.asset).or_default() += note.note.amount;
            acc
        },
    );
    tracing::info!("Shielded balances:");
    for (asset, amount) in balances {
        tracing::info!("{asset}: {amount}");
    }
}

/// Authorizes, proves, and submits `op`, then re-syncs.
async fn operate(session: &mut Session, op: UnsignedOperation) -> anyhow::Result<()> {
    let policy_input = session.authorizer.authorize(&op)?;
    let proven = op.authorize(vec![policy_input])?.prove_with_mmr(
        &session.mmr,
        &session.artifacts,
        &mut OsRng,
    )?;

    send(session, proven.transaction(session.tint_address)).await?;
    sync(session).await?;
    print_balance(session);
    Ok(())
}

/// Fetches events since the last sync, overlapping its final block, and
/// checks the resulting MMR against the chain.
async fn sync(session: &mut Session) -> anyhow::Result<()> {
    // Tint has no known deployment block to start from, so fetch everything
    // in a single `eth_getLogs` call rather than batching from genesis.
    let syncer = RpcSyncer::new(session.provider.clone()).with_batch_size(u64::MAX);
    let from = session.snapshot.range.end.saturating_sub(1);
    let next = syncer.sync(session.tint_address, from..).await?;

    session.mmr.append_events(&next.events)?;
    session.snapshot.extend(next)?;

    // The contract pre-fills its frontier history to avoid cold writes, so an
    // empty frontier never verifies. An empty MMR can't back any proofs anyway.
    if session.mmr.is_empty() {
        return Ok(());
    }
    let verified = session
        .provider
        .tint_verify_mmr(session.tint_address, &session.mmr)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    if !verified {
        anyhow::bail!("local skew-MMR does not match the on-chain frontier");
    }
    Ok(())
}

/// Picks a single spendable note covering `amount` of `asset`. Tint supports
/// spending multiple notes at once, but this CLI keeps things simple and
/// only ever spends one.
fn pick_note(session: &Session, asset: AssetId, amount: u128) -> anyhow::Result<OwnedNote> {
    session
        .snapshot
        .unspent(&session.account)
        .into_iter()
        .find(|note| note.note.asset == asset && note.note.amount >= amount)
        .ok_or_else(|| anyhow::anyhow!("no single spendable note covers {amount} of this asset"))
}

/// Builds a bare (unsigned) provider connection for admin RPC calls that
/// don't require a wallet, such as Tenderly's balance-override cheatcodes.
fn connect_admin(rpc_url: &str) -> anyhow::Result<DynProvider> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    Ok(ProviderBuilder::new()
        .connect_http(rpc_url.parse().context("invalid RPC URL")?)
        .erased())
}

async fn send(session: &Session, tx: TransactionRequest) -> anyhow::Result<()> {
    let tx = tx.with_from(session.from);

    let receipt = session
        .provider
        .send_transaction(tx)
        .await?
        .get_receipt()
        .await?;
    tracing::info!(
        "  tx {} confirmed (gas used: {})",
        receipt.transaction_hash,
        receipt.gas_used
    );
    Ok(())
}

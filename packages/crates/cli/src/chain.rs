use std::sync::Arc;

use alloy::{
    network::TransactionBuilder,
    primitives::{Address, B256, U256},
    providers::{DynProvider, Provider as AlloyProvider, ProviderBuilder},
    rpc::types::TransactionRequest,
    sol,
    sol_types::SolCall,
};
use alloy_provider::ext::TenderlyAdminApi;
use alloy_signer_local::PrivateKeySigner;
use anyhow::Context;
use rand_core::OsRng;
use tint::{
    account::{Account, receiver::Receiver},
    circuit::setup_circuits,
    database::memory::MemoryDatabase,
    indexer::{Indexer, syncer::RpcSyncer, verifier::RpcVerifier},
    note::{asset::AssetId, commitment::SpendableCommitment},
    provider::Provider as TintProvider,
};

use crate::config;

sol! {
    interface IERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
    }
}

/// A signing provider connection plus the Tint state needed to shield,
/// transfer, and unshield funds for a single local account.
pub struct Session {
    provider: DynProvider,
    tint_address: Address,
    from: Address,
    account: Account,
    tint_provider: TintProvider,
}

/// Signs with a local private key and bootstraps the Tint indexer/provider
/// for `account`, mirroring the setup in `tint`'s shield/transfer/unshield
/// integration tests.
pub async fn connect(
    account: Account,
    tint_address: Address,
    rpc_url: &str,
    private_key: B256,
) -> anyhow::Result<Session> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let signer = PrivateKeySigner::from_slice(private_key.as_slice())?;
    let from = signer.address();

    println!("Connecting to {rpc_url}...");
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse().context("invalid RPC URL")?)
        .erased();

    let syncer = Arc::new(RpcSyncer::new(provider.clone(), tint_address));
    let verifier = Arc::new(RpcVerifier::new(provider.clone(), tint_address));
    let database = Arc::new(MemoryDatabase::default());
    let indexer = Indexer::new(syncer, verifier, database).await?;

    let (proving_key, verifying_key) = setup_circuits()?;
    let mut tint_provider = TintProvider::new(indexer, proving_key, verifying_key);
    tint_provider.add_account(account.clone()).await?;

    println!("Syncing with chain...");
    tint_provider.sync().await?;

    Ok(Session {
        provider,
        tint_address,
        from,
        account,
        tint_provider,
    })
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
    println!("tx {tx_hash} confirmed");
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
    println!("tx {tx_hash} confirmed");
    Ok(())
}

/// Auto-approves `token` for `amount` and deposits it into Tint.
pub async fn shield(session: &mut Session, token: Address, amount: u128) -> anyhow::Result<()> {
    let asset = AssetId::from(token);

    println!("Approving Tint to spend {amount} of {token}...");
    let approve_call = IERC20::approveCall {
        spender: session.tint_address,
        amount: U256::from(amount),
    };
    send(session, token, approve_call.abi_encode()).await?;

    println!("Depositing...");
    let call =
        session
            .tint_provider
            .deposit(session.account.receiver(), asset, amount, &mut OsRng)?;
    send(session, session.tint_address, call.abi_encode()).await?;

    session.tint_provider.sync().await?;
    print_balance(session, asset);
    Ok(())
}

/// Transfers `amount` of `token` to `to` (a local account name or an
/// exported shielded address), sending any leftover change back to `account`.
pub async fn transfer(
    session: &mut Session,
    to: &str,
    token: Address,
    amount: u128,
) -> anyhow::Result<()> {
    let asset = AssetId::from(token);
    let recipient = resolve_receiver(to)?;
    let note = pick_note(session, asset, amount)?;
    let change = note.base.amount - amount;
    let sender = session.account.receiver();

    println!("Building transfer proof...");
    let call = if change > 0 {
        session.tint_provider.operate(
            [note],
            [(recipient, asset, amount), (sender, asset, change)],
            [],
            &mut OsRng,
        )?
    } else {
        session
            .tint_provider
            .operate([note], [(recipient, asset, amount)], [], &mut OsRng)?
    };

    send(session, session.tint_address, call.abi_encode()).await?;
    session.tint_provider.sync().await?;
    print_balance(session, asset);
    Ok(())
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
    let note = pick_note(session, asset, amount)?;
    let change = note.base.amount - amount;
    let sender = session.account.receiver();

    println!("Building unshield proof...");
    let call = if change > 0 {
        session.tint_provider.operate(
            [note],
            [(sender, asset, change)],
            [(to, asset, amount)],
            &mut OsRng,
        )?
    } else {
        session
            .tint_provider
            .operate([note], [], [(to, asset, amount)], &mut OsRng)?
    };

    send(session, session.tint_address, call.abi_encode()).await?;
    session.tint_provider.sync().await?;
    print_balance(session, asset);
    Ok(())
}

/// Resolves a transfer recipient: a local account name if one matches,
/// otherwise a hex-encoded exported shielded address.
fn resolve_receiver(to: &str) -> anyhow::Result<Receiver> {
    if let Ok(account) = config::load_account(to) {
        return Ok(account.receiver());
    }

    let bytes = alloy::primitives::hex::decode(to)
        .with_context(|| format!("\"{to}\" is not a known account name or a valid hex address"))?;
    Ok(Receiver::from_address(&bytes)?)
}

/// Picks a single spendable note covering `amount` of `asset`. Tint supports
/// spending multiple notes at once, but this CLI keeps things simple and
/// only ever spends one.
fn pick_note(
    session: &Session,
    asset: AssetId,
    amount: u128,
) -> anyhow::Result<SpendableCommitment> {
    session
        .tint_provider
        .spendable_notes(session.account.receiver())
        .into_iter()
        .find(|note| note.base.asset == asset && note.base.amount >= amount)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no single spendable note covers {amount} of this asset"))
}

fn print_balance(session: &Session, asset: AssetId) {
    let total: u128 = session
        .tint_provider
        .spendable_notes(session.account.receiver())
        .iter()
        .filter(|note| note.base.asset == asset)
        .map(|note| note.base.amount)
        .sum();
    println!("Remaining shielded balance for this asset: {total}");
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

async fn send(session: &Session, to: Address, data: Vec<u8>) -> anyhow::Result<()> {
    let tx = TransactionRequest::default()
        .with_from(session.from)
        .with_to(to)
        .with_input(data);

    let receipt = session
        .provider
        .send_transaction(tx)
        .await?
        .get_receipt()
        .await?;
    println!(
        "  tx {} confirmed (gas used: {})",
        receipt.transaction_hash, receipt.gas_used
    );
    Ok(())
}

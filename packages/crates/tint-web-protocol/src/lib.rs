//! Messages exchanged between the tint-web UI and its tint worker.
//!
//! Both sides send these as JSON strings over `postMessage`.

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

/// A request from the UI, answered by a [`WorkerMessage::Done`] with the same `id`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub kind: RequestKind,
}

/// Account requests name the account by its index among those derived from
/// the connection's private key.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RequestKind {
    /// Syncs and recovers the private key's accounts, replying
    /// [`Reply::Accounts`] with at least `min` of them.
    Accounts { conn: Connection, min: u32 },
    /// Syncs with the chain, replying [`Reply::State`].
    Sync { conn: Connection, account: u32 },
    /// Approves and deposits `amount` of `token`, replying [`Reply::State`].
    Deposit {
        conn: Connection,
        account: u32,
        token: Address,
        amount: u128,
    },
    /// Sends `amount` of `token` to the account with receive address `to`,
    /// replying [`Reply::State`].
    Transfer {
        conn: Connection,
        account: u32,
        to: String,
        token: Address,
        amount: u128,
    },
    /// Withdraws `amount` of `token` to `to`, replying [`Reply::State`].
    Withdraw {
        conn: Connection,
        account: u32,
        to: Address,
        token: Address,
        amount: u128,
    },
}

/// The chain and signer a request runs against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub rpc_url: String,
    pub tint: Address,
    pub private_key: B256,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkerMessage {
    /// The worker is listening for requests.
    Ready,
    /// A status update for an in-flight request.
    Progress { id: u64, message: String },
    /// The outcome of a request.
    Done {
        id: u64,
        result: Result<Reply, String>,
    },
    /// The worker panicked, and can no longer handle requests.
    Crashed { message: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Reply {
    Accounts(Vec<AccountInfo>),
    State(AccountState),
}

/// One of the accounts derived from a private key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountInfo {
    pub index: u32,
    /// The hex-encoded `PubAccount` others send notes to.
    pub receiver: String,
    /// Notes the account has ever received; 0 if it's unused.
    pub notes: usize,
}

/// An account's view of the chain after a sync.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountState {
    /// The address paying for and sending transactions.
    pub signer: Address,
    /// The hex-encoded `PubAccount` others send notes to.
    pub receiver: String,
    /// The exclusive end of the synced block range.
    pub synced_to: u64,
    /// Tint events synced, across all accounts.
    pub events: usize,
    /// Leaves in the note set's skew-MMR.
    pub leaves: u64,
    pub balances: Vec<Balance>,
    /// Operations involving the account, newest first.
    pub history: Vec<HistoryEntry>,
    /// Every note the account has received, newest first.
    pub notes: Vec<NoteInfo>,
    /// Every Tint transaction, from any account, newest first.
    pub global_history: Vec<GlobalEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    pub token: Address,
    pub amount: u128,
    /// The number of unspent notes making up `amount`.
    pub notes: usize,
}

/// One token's movement within an operation involving the account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub block: u64,
    pub tx: B256,
    pub kind: HistoryKind,
    pub token: Address,
    pub amount: u128,
    /// The withdrawal's recipient, for [`HistoryKind::Withdraw`].
    pub recipient: Option<Address>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryKind {
    /// Shielded from a public balance.
    Deposit,
    /// Received from another account's transfer.
    Receive,
    /// Transferred to another account.
    Send,
    /// Unshielded to a public address.
    Withdraw,
    /// Spent notes whose value all came back to the account.
    SelfTransfer,
}

/// What anyone can see of a Tint transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalEntry {
    pub block: u64,
    pub tx: B256,
    /// Notes spent.
    pub nullified: usize,
    /// Notes created, including deposits.
    pub committed: usize,
    pub deposits: Vec<Deposit>,
    pub withdrawals: Vec<Withdrawal>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deposit {
    pub token: Address,
    pub amount: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Withdrawal {
    pub token: Address,
    pub amount: u128,
    pub recipient: Address,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteInfo {
    pub leaf_index: u64,
    pub block: u64,
    pub tx: B256,
    pub token: Address,
    pub amount: u128,
    /// The block the note was spent in, if it has been.
    pub spent_in: Option<u64>,
}

impl RequestKind {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            RequestKind::Accounts { .. } => "accounts",
            RequestKind::Sync { .. } => "sync",
            RequestKind::Deposit { .. } => "deposit",
            RequestKind::Transfer { .. } => "transfer",
            RequestKind::Withdraw { .. } => "withdraw",
        }
    }
}

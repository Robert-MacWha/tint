//! Frame transaction receipts, as ethrex's `eth_getTransactionReceipt` returns them. alloy's
//! Ethereum receipt can't decode type `0x06`.

use alloy_primitives::{Address, B256, U64};
use alloy_rpc_types_eth::Log;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameTxReceipt {
    pub transaction_hash: B256,
    pub block_number: U64,
    pub status: TxStatus,
    pub gas_used: U64,
    /// The account that approved payment.
    pub payer: Option<Address>,
    #[serde(default)]
    pub logs: Vec<Log>,
    #[serde(default)]
    pub frame_receipts: Vec<FrameReceipt>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameReceipt {
    pub status: FrameStatus,
    pub gas_used: U64,
    pub state_gas_used: Option<U64>,
    /// The frame's logs, without the block and transaction fields of the receipt's `logs`.
    #[serde(default)]
    pub logs: Vec<alloy_primitives::Log>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "U64")]
pub enum TxStatus {
    /// At least one frame didn't succeed.
    Failed,
    /// Every frame succeeded.
    Succeeded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "U64")]
pub enum FrameStatus {
    Failed,
    Succeeded,
    /// Not run, or rolled back, because another frame in its atomic batch failed.
    Skipped,
}

#[derive(Debug, thiserror::Error)]
#[error("unknown status {0}")]
pub struct UnknownStatus(U64);

impl TryFrom<U64> for TxStatus {
    type Error = UnknownStatus;

    fn try_from(status: U64) -> Result<Self, Self::Error> {
        match status.to::<u64>() {
            0 => Ok(Self::Failed),
            1 => Ok(Self::Succeeded),
            _ => Err(UnknownStatus(status)),
        }
    }
}

impl TryFrom<U64> for FrameStatus {
    type Error = UnknownStatus;

    fn try_from(status: U64) -> Result<Self, Self::Error> {
        match status.to::<u64>() {
            0 => Ok(Self::Failed),
            1 => Ok(Self::Succeeded),
            2 => Ok(Self::Skipped),
            _ => Err(UnknownStatus(status)),
        }
    }
}

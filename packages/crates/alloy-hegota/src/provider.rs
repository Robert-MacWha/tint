use std::time::Duration;

use alloy_primitives::{Address, B256, Bytes, U64};
use alloy_provider::{Provider, transport::TransportError};
use serde::Deserialize;

use crate::{receipt::FrameTxReceipt, tx::TxFrame};

const RECEIPT_POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, thiserror::Error)]
pub enum HegotaError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("no receipt for {0} after {1:?}")]
    ReceiptTimeout(B256, Duration),
}

/// `ethrex_simulateFrameTransaction`'s result, the frame-native stand-in for
/// `eth_estimateGas`. A transaction the node would refuse comes back with `valid: false`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Simulation {
    pub valid: bool,
    /// Why the transaction is invalid.
    pub violation: Option<String>,
    pub payer: Option<Address>,
    /// `None` if the full execution didn't run.
    pub execution_status: Option<ExecutionStatus>,
    /// Why the full execution couldn't run or complete.
    pub execution_error: Option<String>,
    pub gas_used: Option<U64>,
    /// `None` if the full execution didn't run.
    pub frames: Option<Vec<SimulatedFrame>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// Every frame succeeded.
    Success,
    /// At least one frame didn't succeed.
    Reverted,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatedFrame {
    pub gas_used: U64,
    pub succeeded: bool,
}

/// Frame transaction calls for any alloy provider connected to an ethrex hegota node.
#[async_trait::async_trait]
pub trait HegotaProviderExt: Provider {
    /// Broadcasts a signed frame transaction, returning its hash.
    async fn send_frame_transaction(&self, tx: &TxFrame) -> Result<B256, HegotaError>;

    /// Dry-runs a signed frame transaction.
    async fn simulate_frame_transaction(&self, tx: &TxFrame) -> Result<Simulation, HegotaError>;

    async fn get_frame_receipt(&self, hash: B256) -> Result<Option<FrameTxReceipt>, HegotaError>;

    /// Polls for `hash`'s receipt until it's mined or `timeout` passes.
    async fn watch_frame_receipt(
        &self,
        hash: B256,
        timeout: Duration,
    ) -> Result<FrameTxReceipt, HegotaError>;
}

#[async_trait::async_trait]
impl<P: Provider> HegotaProviderExt for P {
    async fn send_frame_transaction(&self, tx: &TxFrame) -> Result<B256, HegotaError> {
        let raw = Bytes::from(tx.encoded_2718());
        Ok(self
            .raw_request("eth_sendRawTransaction".into(), (raw,))
            .await?)
    }

    async fn simulate_frame_transaction(&self, tx: &TxFrame) -> Result<Simulation, HegotaError> {
        let raw = Bytes::from(tx.encoded_2718());
        Ok(self
            .raw_request("ethrex_simulateFrameTransaction".into(), (raw,))
            .await?)
    }

    async fn get_frame_receipt(&self, hash: B256) -> Result<Option<FrameTxReceipt>, HegotaError> {
        Ok(self
            .raw_request("eth_getTransactionReceipt".into(), (hash,))
            .await?)
    }

    async fn watch_frame_receipt(
        &self,
        hash: B256,
        timeout: Duration,
    ) -> Result<FrameTxReceipt, HegotaError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some(receipt) = self.get_frame_receipt(hash).await? {
                return Ok(receipt);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(HegotaError::ReceiptTimeout(hash, timeout));
            }
            tokio::time::sleep(RECEIPT_POLL_INTERVAL).await;
        }
    }
}

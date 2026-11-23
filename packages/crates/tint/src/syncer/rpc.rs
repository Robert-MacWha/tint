use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::{BlockId, Filter, Log, TransactionRequest};
use alloy_sol_types::{SolCall, SolEvent};
use tracing::{info, warn};

use crate::{
    abis::tint::Tint,
    syncer::{Snapshot, SyncEvent, Syncer, SyncerError},
};

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// A syncer that reads from an Ethereum JSON-RPC provider
///
/// Walks backward through the blocks Tint was active in, using the previously
/// active block each `Checkpoint` event links to.
#[derive(Clone)]
pub struct RpcSyncer<P: Provider> {
    provider: P,
    batch_size: u64,
}

impl<P: Provider> RpcSyncer<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            batch_size: 10_000,
        }
    }

    /// Sets the batch size for `eth_getLogs` calls.
    #[must_use]
    pub fn with_batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = batch_size;
        self
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl<P: Provider> Syncer for RpcSyncer<P> {
    async fn sync_range(
        &self,
        tint: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Snapshot, SyncerError> {
        self.fetch(tint, from_block, to_block)
            .await
            .map_err(SyncerError::Other)
    }
}

impl<P: Provider> RpcSyncer<P> {
    async fn fetch(
        &self,
        tint: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Snapshot, BoxError> {
        //? `latest_block` is an inclusive block number, `to_block` an exclusive bound.
        let latest = self.provider.get_block_number().await?.saturating_add(1);
        let range = from_block..to_block.min(latest).max(from_block);
        info!("Syncing from {} to {}", range.start, range.end);
        if range.is_empty() {
            return Ok(Snapshot::new(tint, range, Vec::new()));
        }

        //? A range that fits in one batch needs no walking back through the
        //? blocks Tint was active in.
        if range.end - range.start <= self.batch_size {
            let (events, _) = self.fetch_window(tint, range.start, range.end - 1).await?;
            return Ok(Snapshot::new(tint, range, events));
        }

        let mut windows = Vec::new();
        let mut cursor = self.head_block(tint, range.end - 1).await?;

        while cursor != 0 && cursor >= range.start {
            let (events, previous) = self.fetch_window(tint, range.start, cursor).await?;
            windows.push(events.into_iter().filter(|e| e.block_number() < range.end));
            info!("{} ({} windows)", cursor, windows.len());
            cursor = previous;
        }

        //? Windows are fetched newest-first, so reverse into emission order.
        let events = windows.into_iter().rev().flatten().collect();
        Ok(Snapshot::new(tint, range, events))
    }

    /// Returns the last block Tint was active in as of `block`.
    ///
    /// Falls back to the latest block if the provider can't serve `block`'s
    /// state.
    async fn head_block(&self, tint: Address, block: u64) -> Result<u64, BoxError> {
        match self.last_active_block(tint, block.into()).await {
            Ok(head) => Ok(head),
            Err(err) => {
                warn!("Failed to read state at block {block}, walking from latest: {err}");
                self.last_active_block(tint, BlockId::latest()).await
            }
        }
    }

    /// Fetches the events in the given window.
    ///
    /// Returns the events and the previous block Tint was active in.
    async fn fetch_window(
        &self,
        tint: Address,
        floor: u64,
        end: u64,
    ) -> Result<(Vec<SyncEvent>, u64), BoxError> {
        let start = end.saturating_sub(self.batch_size.max(1) - 1).max(floor);
        let filter = Filter::new().address(tint).from_block(start).to_block(end);

        //? Each active block emits one checkpoint linking to an earlier block, so
        //? the smallest links to the block preceding the window.
        let mut events = Vec::new();
        let mut previous = None;
        for log in self.provider.get_logs(&filter).await? {
            if let Some(block) = decode_checkpoint(&log) {
                previous = Some(previous.map_or(block, |p: u64| p.min(block)));
                continue;
            }
            let Some(decoded) = SyncEvent::try_from_log(&log) else {
                warn!("Unknown event with topic {:?}", log.topic0());
                continue;
            };
            events.push(decoded);
        }

        Ok((events, previous.unwrap_or(0)))
    }

    async fn last_active_block(&self, tint: Address, block: BlockId) -> Result<u64, BoxError> {
        let call = Tint::checkpointsCall { level: U256::ZERO };
        let tx = TransactionRequest::default()
            .to(tint)
            .input(call.abi_encode().into());
        let result = self.provider.call(tx).block(block).await?;
        Ok(Tint::checkpointsCall::abi_decode_returns(&result)?)
    }
}

fn decode_checkpoint(log: &Log) -> Option<u64> {
    if log.topic0() != Some(&Tint::Checkpoint::SIGNATURE_HASH) {
        return None;
    }
    let decoded = Tint::Checkpoint::decode_log(&log.inner).ok()?;
    Some(decoded.data.previous[0])
}

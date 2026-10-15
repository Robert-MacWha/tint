use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types_eth::Filter;
use tracing::{info, warn};

use crate::syncer::{Snapshot, SyncEvent, Syncer, SyncerError};

/// A syncer that reads from an Ethereum JSON-RPC provider
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

    #[must_use]
    /// Sets the batch size for `eth_getLogs` calls.
    pub fn with_batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = batch_size;
        self
    }
}

#[async_trait::async_trait]
impl<P: Provider> Syncer for RpcSyncer<P> {
    async fn sync_range(
        &self,
        tint: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Snapshot, SyncerError> {
        self.fetch(tint, from_block, to_block)
            .await
            .map_err(SyncerError::other)
    }
}

impl<P: Provider> RpcSyncer<P> {
    async fn fetch(
        &self,
        tint: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Snapshot, alloy_provider::transport::TransportError> {
        //? `latest_block` is an inclusive block number, `to_block` an exclusive bound.
        let latest = self.provider.get_block_number().await?.saturating_add(1);
        let range = from_block..to_block.min(latest).max(from_block);
        info!("Syncing from {} to {}", range.start, range.end);

        let batch_size = self.batch_size.max(1);
        let mut events = Vec::new();
        let mut current = range.start;

        while current < range.end {
            //? `eth_getLogs` treats both block bounds as inclusive
            let batch_end = current.saturating_add(batch_size).min(range.end) - 1;

            let filter = Filter::new()
                .address(tint)
                .from_block(current)
                .to_block(batch_end);

            for log in self.provider.get_logs(&filter).await? {
                let Some(decoded) = SyncEvent::try_from_log(&log) else {
                    warn!("Unknown event with topic {:?}", log.topic0());
                    continue;
                };
                events.push(decoded);
            }

            current = batch_end + 1;
            info!("{}/{} ({} events)", current, range.end, events.len());
        }

        Ok(Snapshot::new(tint, range, events))
    }
}

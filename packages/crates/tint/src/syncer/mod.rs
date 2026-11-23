//! Traits and types for syncing Tint events.

use std::{
    ops::{Bound, Range, RangeBounds},
    sync::Arc,
};

use alloy_primitives::Address;
use thiserror::Error;

mod event;
pub mod rpc;

pub use event::{Committed, Deposited, Nullified, Snapshot, SnapshotError, SyncEvent, Withdrawn};

/// Fetches a Tint deployment's events from some source.
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait Syncer: Send + Sync {
    /// Fetch the events emitted by the Tint deployment at `tint` within the given block range.
    ///
    /// `from_block` and `to_block` form a half-open range. Implementers are expected to
    /// clamp this range to what they can serve. The returned range may be a subset of the requested
    /// range.
    async fn sync_range(
        &self,
        tint: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Snapshot, SyncerError>;

    async fn sync(
        &self,
        tint: Address,
        range: impl RangeBounds<u64> + Send,
    ) -> Result<Snapshot, SyncerError>
    where
        Self: Sized,
    {
        let range = block_range(range);
        self.sync_range(tint, range.start, range.end).await
    }
}

/// A type-erased syncer.
///
/// See [`Syncer`].
#[derive(Clone)]
pub struct DynSyncer(Arc<dyn Syncer>);

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SyncerError {
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl DynSyncer {
    #[must_use]
    pub fn new(syncer: impl Syncer + 'static) -> Self {
        Self(Arc::new(syncer))
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl Syncer for DynSyncer {
    async fn sync_range(
        &self,
        tint: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<Snapshot, SyncerError> {
        self.0.sync_range(tint, from_block, to_block).await
    }
}

impl SyncerError {
    pub fn other<E: std::error::Error + Send + Sync + 'static>(err: E) -> Self {
        Self::Other(Box::new(err))
    }
}

/// Normalizes a block range into a half-open range.
fn block_range(range: impl RangeBounds<u64>) -> Range<u64> {
    let start = match range.start_bound() {
        Bound::Included(&block) => block,
        Bound::Excluded(&block) => block.saturating_add(1),
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&block) => block.saturating_add(1),
        Bound::Excluded(&block) => block,
        Bound::Unbounded => u64::MAX,
    };

    start..end.max(start)
}

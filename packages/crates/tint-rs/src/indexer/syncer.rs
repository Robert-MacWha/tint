use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types_eth::{BlockNumberOrTag, Filter};
use alloy_sol_types::SolEvent;

use crate::abis::{self, tint::Tint};

#[derive(Clone)]
pub enum Event {
    Deposit(abis::tint::Tint::Deposited),
    Committed(abis::tint::Tint::Committed),
    Nullified(abis::tint::Tint::Nullified),
    Withdrawn(abis::tint::Tint::Withdrawn),
}

#[async_trait::async_trait]
pub trait Syncer {
    async fn latest_block(&self)
    -> Result<u64, Box<dyn std::error::Error + Send + Sync + 'static>>;

    async fn sync(
        &self,
        from: u64,
        to: u64,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync + 'static>>;
}

/// A [`Syncer`] backed by an Ethereum JSON-RPC endpoint via `alloy-provider`.
pub struct RpcSyncer<P: Provider> {
    provider: P,
    contract: Address,
}

impl<P: Provider> RpcSyncer<P> {
    pub fn new(provider: P, contract: Address) -> Self {
        Self { provider, contract }
    }
}

#[async_trait::async_trait]
impl<P: Provider> Syncer for RpcSyncer<P> {
    async fn latest_block(
        &self,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(self.provider.get_block_number().await?)
    }

    async fn sync(
        &self,
        from: u64,
        to: u64,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let filter = Filter::new()
            .address(self.contract)
            .from_block(BlockNumberOrTag::Number(from))
            .to_block(BlockNumberOrTag::Number(to));

        let logs = self.provider.get_logs(&filter).await?;

        let mut events = Vec::with_capacity(logs.len());
        for log in logs {
            let Some(topic0) = log.topic0().copied() else {
                continue;
            };

            let event = match topic0 {
                Tint::Deposited::SIGNATURE_HASH => {
                    Event::Deposit(log.log_decode::<Tint::Deposited>()?.inner.data)
                }
                Tint::Committed::SIGNATURE_HASH => {
                    Event::Committed(log.log_decode::<Tint::Committed>()?.inner.data)
                }
                Tint::Nullified::SIGNATURE_HASH => {
                    Event::Nullified(log.log_decode::<Tint::Nullified>()?.inner.data)
                }
                Tint::Withdrawn::SIGNATURE_HASH => {
                    Event::Withdrawn(log.log_decode::<Tint::Withdrawn>()?.inner.data)
                }
                _ => continue,
            };
            events.push(event);
        }

        Ok(events)
    }
}

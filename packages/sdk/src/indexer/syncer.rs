use crate::abis;

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

use std::sync::Arc;

pub mod syncer;
pub mod verifier;

pub struct Indexer {
    syncer: Arc<dyn syncer::Syncer>,
    verifier: Arc<dyn verifier::Verifier>,
}

impl Indexer {
    pub fn new(syncer: Arc<dyn syncer::Syncer>, verifier: Arc<dyn verifier::Verifier>) -> Self {
        Self { syncer, verifier }
    }
}

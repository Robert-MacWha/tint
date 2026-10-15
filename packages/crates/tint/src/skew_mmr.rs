use ark_bn254::Fr;
use tint_poseidon2::skew_mmr::Poseidon2Hasher;

pub use crate::circuit::join_split::MMR_MAX_DEPTH;
use crate::syncer::SyncEvent;

/// Skew MMR for tint.
pub type SkewMmr = skew_mmr::SkewMmr<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>;

/// Skew MMR proof for tint.
pub type SkewMmrProof = skew_mmr::proof::Proof<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>;

/// `SkewMmr` extension trait for Tint.
pub trait SkewMmrExt {
    /// Appends the commitments carried by `events`, in leaf order.
    ///
    /// Commitments at leaf indices already in the MMR are skipped, so
    /// overlapping snapshots may be re-applied. Skipped commitments are not
    /// checked against the MMR's leaves; a reorg that replaced them is only
    /// caught by verifying the MMR against the chain.
    ///
    /// # Errors
    ///
    /// Errors if a commitment's leaf index leaves a gap in the MMR.
    fn append_events(&mut self, events: &[SyncEvent]) -> Result<(), AppendEventsError>;
}

#[derive(Debug, thiserror::Error)]
#[error("non-contiguous commitment events: expected leaf {expected}, got {got}")]
pub struct AppendEventsError {
    pub expected: u64,
    pub got: u64,
}

impl SkewMmrExt for SkewMmr {
    fn append_events(&mut self, events: &[SyncEvent]) -> Result<(), AppendEventsError> {
        for (leaf_index, commitment) in events.iter().filter_map(SyncEvent::commitment) {
            let expected = self.len() as u64;
            if leaf_index < expected {
                continue;
            }
            if leaf_index > expected {
                return Err(AppendEventsError {
                    expected,
                    got: leaf_index,
                });
            }
            self.append(commitment);
        }
        Ok(())
    }
}

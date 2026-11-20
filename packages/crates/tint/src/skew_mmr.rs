use ark_bn254::Fr;
use tint_poseidon2::skew_mmr::Poseidon2Hasher;

pub use crate::circuit::join_split::MMR_MAX_DEPTH;

/// Skew MMR for tint.
pub type SkewMmr = skew_mmr::SkewMmr<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>;

/// Skew MMR proof for tint.
pub type SkewMmrProof = skew_mmr::proof::Proof<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>;

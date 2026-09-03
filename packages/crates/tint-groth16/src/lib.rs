//! Groth16 prover & proving artifacts
//!
//! Consider removing / replacing parts of this with `taceo_groth16`. We aren't doing
//! that right now because their matrix type isn't serializable, but otherwise it'd
//! be a good replacement.

pub mod artifacts;
pub mod groth16;
pub mod matrices;
pub mod prove;
pub mod serde;

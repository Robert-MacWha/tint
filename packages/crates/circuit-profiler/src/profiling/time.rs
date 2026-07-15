//! Profiling by elapsed wall-clock time.

use std::time::{Duration, Instant};

use ark_ff::Field;
use ark_relations::gr1cs::{ConstraintSystemRef, SynthesisError};

use crate::profiling::{
    ConstraintStats, core,
    gr1cs::{ConstraintProfile, R1CS_TARGET, new_cs},
};

/// Synthesizes `f` against a fresh constraint system while recording a
/// per-span elapsed-time call tree, via every `target = "r1cs"` tracing
/// span active during synthesis.
pub fn profile_time<F: Field>(
    f: impl FnOnce(ConstraintSystemRef<F>) -> Result<(), SynthesisError>,
) -> ConstraintProfile<Duration> {
    let cs = new_cs::<F>();

    let start = Instant::now();
    let nodes = core::profile(
        R1CS_TARGET,
        move || start.elapsed(),
        || {
            f(cs.clone()).unwrap();
        },
    );
    cs.finalize();

    ConstraintProfile {
        stats: ConstraintStats::from_cs(&cs),
        nodes,
    }
}

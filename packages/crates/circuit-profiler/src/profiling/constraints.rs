//! Profiling by constraint count.

use std::cell::RefCell;

use ark_ff::Field;
use ark_relations::gr1cs::{ConstraintSystemRef, SynthesisError};

use crate::profiling::{
    ConstraintStats, core,
    gr1cs::{ConstraintProfile, R1CS_TARGET, new_cs},
};

thread_local! {
    // `ConstraintSystemRef` isn't `Send`/`Sync`, so it can't be captured by the
    // `sample` closure passed to `core::profile`.
    static ACTIVE_CS: RefCell<Option<Box<dyn Fn() -> usize>>> = const { RefCell::new(None) };
}

/// Synthesizes `f` against a fresh constraint system while recording a
/// per-span constraint-count call tree.
pub fn profile_constraints<F: Field>(
    f: impl FnOnce(ConstraintSystemRef<F>) -> Result<(), SynthesisError>,
) -> ConstraintProfile<usize> {
    let cs = new_cs::<F>();

    let active_cs = cs.clone();
    ACTIVE_CS.with(|active| {
        *active.borrow_mut() = Some(Box::new(move || active_cs.num_constraints()));
    });

    let nodes = core::profile(
        R1CS_TARGET,
        || ACTIVE_CS.with(|active| active.borrow().as_ref().map(|sample| sample()).unwrap_or(0)),
        || {
            f(cs.clone()).unwrap();
        },
    );

    ACTIVE_CS.with(|active| *active.borrow_mut() = None);
    cs.finalize();

    ConstraintProfile {
        stats: ConstraintStats::from_cs(&cs),
        nodes,
    }
}

//! Scaffolding shared by the arkworks-specific profilers in [`super::constraints`]
//! and [`super::time`]: building a fresh constraint system and reading its
//! headline stats once synthesis has finished.

use ark_ff::Field;
use ark_relations::gr1cs::{ConstraintSystem, ConstraintSystemRef, OptimizationGoal};

use crate::profiling::Node;

pub(crate) const R1CS_TARGET: &str = "r1cs";

/// The result of profiling a circuit's synthesis: headline stats for the
/// synthesized constraint system, plus a per-span call tree of whatever
/// metric was profiled (constraint count, elapsed time, ...).
pub struct ConstraintProfile<T> {
    pub stats: ConstraintStats,
    pub nodes: Vec<Node<T>>,
}

/// Headline stats for a synthesized constraint system.
#[derive(Debug, Clone, Copy)]
pub struct ConstraintStats {
    pub num_constraints: usize,
    pub num_instance_variables: usize,
    pub num_witness_variables: usize,
    pub num_variables: usize,
    pub num_predicates: usize,
}

pub fn new_cs<F: Field>() -> ConstraintSystemRef<F> {
    let cs = ConstraintSystem::<F>::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);
    cs
}

impl ConstraintStats {
    pub fn from_cs<F: Field>(cs: &ConstraintSystemRef<F>) -> Self {
        ConstraintStats {
            num_constraints: cs.num_constraints(),
            num_instance_variables: cs.num_instance_variables(),
            num_witness_variables: cs.num_witness_variables(),
            num_variables: cs.num_variables(),
            num_predicates: cs.num_predicates(),
        }
    }
}

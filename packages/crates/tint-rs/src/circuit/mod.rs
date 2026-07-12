use ark_ff::Field;
use ark_r1cs_std::alloc::AllocVar;

pub mod commitment;
pub mod join_split;
pub mod merkle_tree;
pub mod operation;
pub mod poseidon;

pub type FrVar = ark_r1cs_std::fields::fp::FpVar<ark_bn254::Fr>;

/// Helper to create a new variable in the constraint system with the given
/// value and allocation mode.
fn variable<T, F: Field, TVar: AllocVar<T, F>>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
    mode: ark_r1cs_std::prelude::AllocationMode,
) -> Result<TVar, ark_relations::gr1cs::SynthesisError> {
    TVar::new_variable(cs, || Ok(value), mode)
}

/// Helper to create a new witness variable in the constraint system with the given
/// value.
#[allow(dead_code)]
fn witness<T, F: Field, TVar: AllocVar<T, F>>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
) -> Result<TVar, ark_relations::gr1cs::SynthesisError> {
    variable(cs, value, ark_r1cs_std::prelude::AllocationMode::Witness)
}

/// Helper to create a new constant variable in the constraint system with the given
/// value.
#[allow(dead_code)]
fn constant<T, F: Field, TVar: AllocVar<T, F>>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
) -> Result<TVar, ark_relations::gr1cs::SynthesisError> {
    TVar::new_constant(cs, value)
}

/// Creates an array of size N by trying to call the provided fn for each index.
fn try_array_from_fn<T, E, const N: usize>(
    mut f: impl FnMut(usize) -> Result<T, E>,
) -> Result<[T; N], E> {
    Ok((0..N)
        .map(&mut f)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .unwrap_or_else(|_| unreachable!()))
}

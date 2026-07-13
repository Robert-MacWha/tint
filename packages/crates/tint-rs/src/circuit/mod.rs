use ark_ff::Field;
use ark_r1cs_std::{GR1CSVar, alloc::AllocVar};

pub mod commitment;
pub mod join_split;
pub mod merkle_tree;
pub mod operation;
pub mod poseidon;
pub mod poseidon2;

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

/// Helper to create a new public input variable in the constraint system
/// with the given value.
fn input<T, F: Field, TVar: AllocVar<T, F>>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
) -> Result<TVar, ark_relations::gr1cs::SynthesisError> {
    variable(cs, value, ark_r1cs_std::prelude::AllocationMode::Input)
}

/// Helper to create a new public output variable in the constraint system
///
/// Public outputs are emulated by creating a new public input variable and
/// enforcing that it is equal to the value computed in-circuit.
fn output<F, T>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
) -> Result<(), ark_relations::gr1cs::SynthesisError>
where
    F: Field,
    T: GR1CSVar<F> + AllocVar<<T as GR1CSVar<F>>::Value, F> + ark_r1cs_std::eq::EqGadget<F>,
{
    let out = T::new_input(cs, || value.value())?;
    out.enforce_equal(value)?;
    Ok(())
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

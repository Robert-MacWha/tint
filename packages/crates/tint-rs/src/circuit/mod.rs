use ark_bn254::Bn254;
use ark_ff::Field;
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_r1cs_std::{GR1CSVar, alloc::AllocVar};
use ark_snark::SNARK;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tracing::{info, warn};

use crate::circuit::join_split::JoinSplit;

pub mod commitment;
pub mod join_split;
pub mod merkle_tree;
pub mod operation;
pub mod poseidon2;

pub type FrVar = ark_r1cs_std::fields::fp::FpVar<ark_bn254::Fr>;

/// Sets up the circuits and returns the proving and verifying keys.
///
/// This circuit setup is deterministic using a fixed seed. It is not cryptographically
/// secure and should only be used for testing and development.
pub fn setup_circuits()
-> Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>), ark_relations::gr1cs::SynthesisError> {
    let mut rng = StdRng::seed_from_u64(1);

    warn!("Circuit setup with fixed seed. Only use for testing and development.");
    let circuit = JoinSplit::default();
    let (proving_key, verifying_key) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)?;

    Ok((proving_key, verifying_key))
}

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

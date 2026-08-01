use ark_bn254::{Bn254, Fr};
use ark_ff::Field;
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_r1cs_std::{GR1CSVar, alloc::AllocVar};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, R1CS_PREDICATE_LABEL, SynthesisError,
};
use ark_snark::SNARK;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tracing::{info, warn};

use crate::circuit::matrices::Matrices;

pub mod artifacts;
pub mod commitment;
pub mod join_split;
pub mod matrices;
pub mod merkle_tree;
pub mod operation;
pub mod poseidon2;

pub type FrVar = ark_r1cs_std::fields::fp::FpVar<ark_bn254::Fr>;

/// Sets up the circuit `C` and returns its proving and verifying keys.
///
/// This circuit setup is deterministic using a fixed seed. It is not cryptographically
/// secure and should only be used for testing and development.
pub fn generate_artifacts<C: ConstraintSynthesizer<Fr> + Default>()
-> Result<(Matrices, ProvingKey<Bn254>, VerifyingKey<Bn254>), ark_relations::gr1cs::SynthesisError>
{
    let mut rng = StdRng::seed_from_u64(42);

    // Generate proving and verifying keys
    warn!("Circuit setup with fixed seed. Only use for testing and development.");
    let (proving_key, verifying_key) =
        Groth16::<Bn254>::circuit_specific_setup(C::default(), &mut rng)?;

    // Generate constraint matrices
    let matrices = generate_matrices::<C>()?;

    info!("Circuit setup complete.");
    Ok((matrices, proving_key, verifying_key))
}

/// Generates the constraint matrices for the circuit `C`.
fn generate_matrices<C: ConstraintSynthesizer<Fr> + Default>() -> Result<Matrices, SynthesisError> {
    let cs = ConstraintSystem::new_ref();
    cs.set_optimization_goal(ark_relations::gr1cs::OptimizationGoal::Constraints);
    cs.set_mode(ark_relations::gr1cs::SynthesisMode::Prove {
        construct_matrices: false,
        generate_lc_assignments: false,
    });
    C::default().generate_constraints(cs.clone())?;
    let matrices = cs
        .to_matrices()?
        .get(R1CS_PREDICATE_LABEL)
        .ok_or(SynthesisError::MissingCS)?
        .clone();
    let num_inputs = cs.num_instance_variables();
    let num_constraints = cs.num_constraints();
    let num_witness_variables = cs.num_witness_variables();
    let matrices = Matrices::new(matrices, num_inputs, num_constraints, num_witness_variables);
    Ok(matrices)
}

/// Helper to create a new constant variable in the constraint system with the given
/// value.
#[allow(dead_code)]
pub fn constant<T, F: Field, TVar: AllocVar<T, F>>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
) -> Result<TVar, ark_relations::gr1cs::SynthesisError> {
    TVar::new_constant(cs, value)
}

/// Helper to create a new public input variable in the constraint system
/// with the given value.
pub fn input<T, F: Field, TVar: AllocVar<T, F>>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
) -> Result<TVar, ark_relations::gr1cs::SynthesisError> {
    variable(cs, value, ark_r1cs_std::prelude::AllocationMode::Input)
}

/// Helper to create a new public output variable in the constraint system
///
/// Public outputs are emulated by creating a new public input variable and
/// enforcing that it is equal to the value computed in-circuit.
pub fn output<F, T>(
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

/// Helper to create a new witness variable in the constraint system with the given
/// value.
#[allow(dead_code)]
pub fn witness<T, F: Field, TVar: AllocVar<T, F>>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
) -> Result<TVar, ark_relations::gr1cs::SynthesisError> {
    variable(cs, value, ark_r1cs_std::prelude::AllocationMode::Witness)
}

/// Helper to create a new variable in the constraint system with the given
/// value and allocation mode.
pub fn variable<T, F: Field, TVar: AllocVar<T, F>>(
    cs: impl Into<ark_relations::gr1cs::Namespace<F>>,
    value: &T,
    mode: ark_r1cs_std::prelude::AllocationMode,
) -> Result<TVar, ark_relations::gr1cs::SynthesisError> {
    TVar::new_variable(cs, || Ok(value), mode)
}

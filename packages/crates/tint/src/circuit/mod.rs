use ark_bn254::{Bn254, Fr};
use ark_ec::pairing::Pairing;
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

#[derive(Clone, Debug)]
pub struct Artifacts<E: Pairing> {
    pub matrices: Matrices<E::ScalarField>,
    pub pk: ProvingKey<E>,
    pub vk: VerifyingKey<E>,
}

/// Sets up the circuit `C` and returns its proving and verifying keys.
///
/// This circuit setup is deterministic using a fixed seed. It is not cryptographically
/// secure and should only be used for testing and development.
pub fn generate_artifacts<C: ConstraintSynthesizer<Fr> + Default>()
-> Result<Artifacts<Bn254>, ark_relations::gr1cs::SynthesisError> {
    let mut rng = StdRng::seed_from_u64(42);

    warn!("Circuit setup with fixed seed. Only use for testing and development.");
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(C::default(), &mut rng)?;
    let matrices = generate_matrices::<C>()?;

    info!("Circuit setup complete.");
    Ok(Artifacts { matrices, pk, vk })
}

/// Generates the constraint matrices for the circuit `C`.
fn generate_matrices<C: ConstraintSynthesizer<Fr> + Default>()
-> Result<Matrices<Fr>, SynthesisError> {
    let cs = ConstraintSystem::new_ref();
    cs.set_optimization_goal(ark_relations::gr1cs::OptimizationGoal::Constraints);
    cs.set_mode(ark_relations::gr1cs::SynthesisMode::Prove {
        construct_matrices: true,
        generate_lc_assignments: false,
    });

    C::default().generate_constraints(cs.clone())?;
    cs.finalize();

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

#[cfg(test)]
mod tests {
    use ark_r1cs_std::eq::EqGadget;
    use ark_relations::gr1cs::ConstraintSystemRef;

    use super::*;
    use crate::circuit::matrices::prove_with_matrices;

    #[derive(Clone, Default)]
    struct XEqualsY {
        x: Fr,
        y: Fr,
    }

    impl ConstraintSynthesizer<Fr> for XEqualsY {
        fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
            let x: FrVar = input(cs.clone(), &self.x)?;
            let y: FrVar = witness(cs, &self.y)?;
            x.enforce_equal(&y)?;
            Ok(())
        }
    }

    /// Regression test: matrices generated by `generate_matrices` must be
    /// usable to produce a proof that verifies against the keys produced by
    /// the same circuit's setup.
    #[test]
    fn generated_matrices_produce_valid_proof() {
        let artifacts = generate_artifacts::<XEqualsY>().unwrap();

        let circuit = XEqualsY {
            x: Fr::from(5u64),
            y: Fr::from(5u64),
        };
        let mut rng = StdRng::seed_from_u64(1);
        let (public_inputs, proof) =
            prove_with_matrices(&artifacts.matrices, &artifacts.pk, &circuit, &mut rng).unwrap();

        assert!(Groth16::<Bn254>::verify(&artifacts.vk, &public_inputs, &proof).unwrap());
    }
}

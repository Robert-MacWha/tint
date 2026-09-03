use ark_ec::pairing::Pairing;
use ark_ff::UniformRand;
use ark_groth16::{Proof, ProvingKey};
use ark_relations::gr1cs::{ConstraintSynthesizer, ConstraintSystem, SynthesisError};
use rand_core::{CryptoRng, RngCore};

use crate::matrices::Matrices;

/// Proves a circuit using pre-computed constraint matrices.
///
/// Returns the public inputs and the proof.
pub fn prove_with_matrices<
    E: Pairing,
    C: ConstraintSynthesizer<E::ScalarField> + Clone,
    R: RngCore + CryptoRng,
>(
    matrices: &Matrices<E::ScalarField>,
    pk: &ProvingKey<E>,
    circuit: &C,
    rng: &mut R,
) -> Result<(Vec<E::ScalarField>, Proof<E>), SynthesisError> {
    let r = E::ScalarField::rand(rng);
    let s = E::ScalarField::rand(rng);

    let cs = ConstraintSystem::new_ref();
    cs.set_optimization_goal(ark_relations::gr1cs::OptimizationGoal::Constraints);
    cs.set_mode(ark_relations::gr1cs::SynthesisMode::Prove {
        construct_matrices: false,
        generate_lc_assignments: false,
    });
    circuit.clone().generate_constraints(cs.clone())?;

    let public_inputs = cs.instance_assignment()?[1..].to_vec();
    let full_assignment = [cs.instance_assignment()?, cs.witness_assignment()?].concat();

    let proof = crate::groth16::Groth16::prove::<crate::groth16::LibSnarkReduction>(
        pk,
        r,
        s,
        matrices,
        &full_assignment,
    )?;

    Ok((public_inputs, proof))
}

use ark_bn254::{Bn254, Fr};
use ark_ff::UniformRand;
use ark_groth16::{Groth16, Proof, ProvingKey};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, R1CS_PREDICATE_LABEL,
    SynthesisError,
};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

/// Constraint matrices for a circuit, including the number of inputs, constraints, and witness variables.
///
/// Can be pre-computed for more efficient proving.
#[serde_with::serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Matrices {
    #[serde_as(as = "Vec<Vec<Vec<(crate::serde::fr::FrAsBytes, _)>>>")]
    pub matrices: Vec<Vec<Vec<(Fr, usize)>>>,
    pub num_inputs: usize,
    pub num_constraints: usize,
    pub num_witness_variables: usize,
}

/// Proves a circuit using pre-computed constraint matrices.
pub fn prove_with_matrices<C: ConstraintSynthesizer<Fr> + Clone, R: RngCore + CryptoRng>(
    pk: &ProvingKey<Bn254>,
    circuit: C,
    matrices: &Matrices,
    rng: &mut R,
) -> Result<Proof<Bn254>, SynthesisError> {
    let r = Fr::rand(rng);
    let s = Fr::rand(rng);

    let cs = ConstraintSystem::new_ref();
    cs.set_optimization_goal(ark_relations::gr1cs::OptimizationGoal::Constraints);
    cs.set_mode(ark_relations::gr1cs::SynthesisMode::Prove {
        construct_matrices: false,
        generate_lc_assignments: false,
    });
    circuit.clone().generate_constraints(cs.clone())?;

    let full_assignment = [cs.instance_assignment()?, cs.witness_assignment()?].concat();

    Groth16::<Bn254>::create_proof_with_reduction_and_matrices(
        pk,
        r,
        s,
        &matrices.matrices,
        matrices.num_inputs,
        matrices.num_constraints,
        &full_assignment,
    )
}

impl Matrices {
    pub fn new(
        matrices: Vec<Vec<Vec<(Fr, usize)>>>,
        num_inputs: usize,
        num_constraints: usize,
        num_witness_variables: usize,
    ) -> Self {
        Self {
            matrices,
            num_inputs,
            num_constraints,
            num_witness_variables,
        }
    }

    pub fn from_constraint_system(cs: &ConstraintSystemRef<Fr>) -> Result<Self, SynthesisError> {
        let matrices = cs.to_matrices()?[R1CS_PREDICATE_LABEL].clone();
        let num_inputs = cs.num_instance_variables();
        let num_constraints = cs.num_constraints();
        let num_witness_variables = cs.num_witness_variables();

        Ok(Self {
            matrices,
            num_inputs,
            num_constraints,
            num_witness_variables,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_deserialize_matrices() {
        let matrices = Matrices {
            matrices: vec![
                vec![
                    vec![(Fr::from(1u64), 0), (Fr::from(2u64), 1)],
                    vec![(Fr::from(3u64), 2)],
                ],
                vec![vec![(Fr::from(4u64), 3)]],
            ],
            num_inputs: 5,
            num_constraints: 6,
            num_witness_variables: 7,
        };

        let serialized = serde_json::to_string(&matrices).unwrap();
        let deserialized: Matrices = serde_json::from_str(&serialized).unwrap();

        assert_eq!(matrices, deserialized);
    }
}

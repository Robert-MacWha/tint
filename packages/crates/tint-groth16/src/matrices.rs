use ark_ff::PrimeField;
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, R1CS_PREDICATE_LABEL,
    SynthesisError,
};
use serde::{Deserialize, Serialize};

#[serde_with::serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Matrices<F: PrimeField> {
    #[serde_as(as = "Vec<Vec<Vec<(crate::serde::field::FieldAsBytes, _)>>>")]
    pub matrices: Vec<Vec<Vec<(F, usize)>>>,
    pub num_inputs: usize,
    pub num_constraints: usize,
    pub num_witness_variables: usize,
}

impl<F: PrimeField> Matrices<F> {
    #[must_use]
    pub fn new(
        matrices: Vec<Vec<Vec<(F, usize)>>>,
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

    /// Generates the constraint matrices for the generic circuit `C`.
    #[must_use]
    pub fn generate<C: ConstraintSynthesizer<F> + Default>() -> Result<Self, SynthesisError> {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(ark_relations::gr1cs::OptimizationGoal::Constraints);
        cs.set_mode(ark_relations::gr1cs::SynthesisMode::Prove {
            construct_matrices: true,
            generate_lc_assignments: false,
        });

        C::default().generate_constraints(cs.clone())?;
        cs.finalize();

        let matrices = cs.try_into()?;
        Ok(matrices)
    }
}

impl<F: PrimeField> TryFrom<ConstraintSystemRef<F>> for Matrices<F> {
    type Error = SynthesisError;

    fn try_from(value: ConstraintSystemRef<F>) -> Result<Self, Self::Error> {
        let matrices = value
            .to_matrices()?
            .get(R1CS_PREDICATE_LABEL)
            .ok_or(SynthesisError::MissingCS)?
            .clone();
        let num_inputs = value.num_instance_variables();
        let num_constraints = value.num_constraints();
        let num_witness_variables = value.num_witness_variables();
        Ok(Matrices::new(
            matrices,
            num_inputs,
            num_constraints,
            num_witness_variables,
        ))
    }
}

#[cfg(test)]
mod tests {
    use ark_bn254::Fr;

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
        let deserialized: Matrices<Fr> = serde_json::from_str(&serialized).unwrap();

        assert_eq!(matrices, deserialized);
    }
}

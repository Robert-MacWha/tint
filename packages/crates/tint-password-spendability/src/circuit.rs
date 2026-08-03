use ark_bn254::Fr;
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, OptimizationGoal, SynthesisError,
};
use tint::{
    circuit::{
        FrVar, input,
        join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
        operation::OperationVar,
        poseidon2::poseidon2_compress_gadget,
        variable, witness,
    },
    operation::Operation,
};

/// Spendability circuit for the "Password" rule, which proves knowledge of a
/// private key bound to a note's spendability hash for a given operation.
#[derive(Clone, Default)]
pub struct PasswordSpendability {
    pub spendability_address: Fr,
    pub operation_hash: Fr,

    // Witnessed values
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub password: Fr,
}

pub struct PasswordSpendabilityVar {
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub secret: FrVar,
}

impl PasswordSpendability {
    pub fn new(
        spendability_address: Fr,
        operation_hash: Fr,
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
        secret: Fr,
    ) -> Self {
        Self {
            spendability_address,
            operation_hash,
            operation,
            password: secret,
        }
    }

    pub fn synthesize_public_inputs(&self) -> Result<Vec<Fr>, SynthesisError> {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);

        self.synthesize(&cs)?;
        cs.finalize();

        // `instance_assignment()` leads with the implicit constant-1 term.
        Ok(cs.instance_assignment()?[1..].to_vec())
    }

    fn synthesize(&self, cs: &ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public inputs
        let spendability_address: FrVar = input(cs.clone(), &self.spendability_address)?;
        let operation_hash: FrVar = input(cs.clone(), &self.operation_hash)?;

        // Witnessed values
        let password_spendability_var: PasswordSpendabilityVar = witness(cs.clone(), self)?;
        password_spendability_var.verify(&operation_hash, &spendability_address)?;

        Ok(())
    }
}

impl ConstraintSynthesizer<Fr> for PasswordSpendability {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        self.synthesize(&cs)
    }
}

impl AllocVar<PasswordSpendability, Fr> for PasswordSpendabilityVar {
    fn new_variable<T: std::borrow::Borrow<PasswordSpendability>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let operation = variable(cs.clone(), &value.operation, mode)?;
        let secret = variable(cs.clone(), &value.password, mode)?;
        Ok(Self { operation, secret })
    }
}

impl PasswordSpendabilityVar {
    #[tracing::instrument(target = "gr1cs", skip_all)]
    pub fn verify(
        &self,
        operation_hash: &FrVar,
        spendability_address: &FrVar,
    ) -> Result<(), SynthesisError> {
        self.verify_operation_hash(operation_hash)?;
        self.verify_spendability_witnesses(spendability_address)?;

        Ok(())
    }

    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn verify_operation_hash(&self, operation_hash: &FrVar) -> Result<(), SynthesisError> {
        let computed_operation_hash = self.operation.hash()?;
        computed_operation_hash.enforce_equal(operation_hash)
    }

    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn verify_spendability_witnesses(
        &self,
        spendability_address: &FrVar,
    ) -> Result<(), SynthesisError> {
        for input in &self.operation.inputs {
            let spendability_addresses_eq =
                input.spendability_address.is_eq(spendability_address)?;
            let expected_witness = poseidon2_compress_gadget(core::array::from_ref(&self.secret))?;
            input
                .spendability_witness
                .conditional_enforce_equal(&expected_witness, &spendability_addresses_eq)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;
    use ark_relations::gr1cs::trace::{ConstraintLayer, TracingMode};
    use tint::{circuit::poseidon2::poseidon2_compress, fr::address_to_fr};
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;

    fn setup_constraint_tracing() -> tracing::subscriber::DefaultGuard {
        let mut layer = ConstraintLayer::default();
        layer.mode = TracingMode::OnlyConstraints;
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        tracing::subscriber::set_default(subscriber)
    }

    #[test]
    fn valid_circuit() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let spendability_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);

        let mut operation = Operation::default();
        operation.inputs[0].spendability_address = spendability_address;
        operation.inputs[0].spendability_witness = witness;

        let operation_hash = operation.hash();

        let circuit = PasswordSpendability::new(
            address_to_fr(spendability_address),
            operation_hash,
            operation,
            secret,
        );
        circuit.synthesize(&cs).unwrap();
        assert!(cs.is_satisfied().unwrap())
    }

    #[test]
    fn multiple_inputs() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let spendability_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);

        let mut operation = Operation::default();
        operation.inputs[0].spendability_address = spendability_address;
        operation.inputs[0].spendability_witness = witness;
        operation.inputs[1].spendability_address = spendability_address;
        operation.inputs[1].spendability_witness = witness;

        let operation_hash = operation.hash();

        let circuit = PasswordSpendability::new(
            address_to_fr(spendability_address),
            operation_hash,
            operation,
            secret,
        );
        circuit.synthesize(&cs).unwrap();
        assert!(cs.is_satisfied().unwrap())
    }

    #[test]
    fn invalid_secret() {
        let _guard = setup_constraint_tracing();
        let cs = ConstraintSystem::<Fr>::new_ref();

        let spendability_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);
        let invalid_secret = Fr::from(54321);

        let mut operation = Operation::default();
        operation.inputs[0].spendability_address = spendability_address;
        operation.inputs[0].spendability_witness = witness;

        let operation_hash = operation.hash();

        let circuit = PasswordSpendability::new(
            address_to_fr(spendability_address),
            operation_hash,
            operation,
            invalid_secret,
        );
        circuit.synthesize(&cs).unwrap();

        let failed = cs
            .which_is_unsatisfied()
            .unwrap()
            .expect("expected some unsatisfied constraints");
        assert!(
            failed.contains("verify_spendability_witnesses"),
            "expected failure in verify_spendability_witnesses, got:\n{failed}"
        );
    }

    #[test]
    fn multiple_inputs_invalid_secret() {
        let _guard = setup_constraint_tracing();
        let cs = ConstraintSystem::<Fr>::new_ref();

        let spendability_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);
        let other_witness = poseidon2_compress(&[Fr::from(54321)]);

        let mut operation = Operation::default();
        operation.inputs[0].spendability_address = spendability_address;
        operation.inputs[0].spendability_witness = witness;
        operation.inputs[1].spendability_address = spendability_address;
        operation.inputs[1].spendability_witness = other_witness;

        let operation_hash = operation.hash();

        let circuit = PasswordSpendability::new(
            address_to_fr(spendability_address),
            operation_hash,
            operation,
            secret,
        );
        circuit.synthesize(&cs).unwrap();

        let failed = cs
            .which_is_unsatisfied()
            .unwrap()
            .expect("expected some unsatisfied constraints");
        assert!(
            failed.contains("verify_spendability_witnesses"),
            "expected failure in verify_spendability_witnesses, got:\n{failed}"
        );
    }

    #[test]
    fn invalid_operation_hash() {
        let _guard = setup_constraint_tracing();
        let cs = ConstraintSystem::<Fr>::new_ref();

        let spendability_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);

        let mut operation = Operation::default();
        operation.inputs[0].spendability_address = spendability_address;
        operation.inputs[0].spendability_witness = witness;

        let operation_hash = operation.hash();
        let invalid_operation_hash = operation_hash + Fr::from(1);

        let circuit = PasswordSpendability::new(
            address_to_fr(spendability_address),
            invalid_operation_hash,
            operation,
            secret,
        );
        circuit.synthesize(&cs).unwrap();

        let failed = cs
            .which_is_unsatisfied()
            .unwrap()
            .expect("expected some unsatisfied constraints");
        assert!(
            failed.contains("verify_operation_hash"),
            "expected failure in verify_operation_hash, got:\n{failed}"
        );
    }
}

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
/// private key bound to a note's spendability hash for a given operation's
/// signing hash.
#[derive(Clone, Default)]
pub struct PasswordSpendability {
    pub spendability_address: Fr,
    pub signing_hash: Fr,

    // Witnessed values
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub context_hash: Fr,
    pub password: Fr,
}

pub struct PasswordSpendabilityVar {
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub context_hash: FrVar,
    pub secret: FrVar,
}

impl PasswordSpendability {
    pub fn new(
        spendability_address: Fr,
        signing_hash: Fr,
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
        context_hash: Fr,
        secret: Fr,
    ) -> Self {
        Self {
            spendability_address,
            signing_hash,
            operation,
            context_hash,
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
        let signing_hash: FrVar = input(cs.clone(), &self.signing_hash)?;

        // Witnessed values
        let password_spendability_var: PasswordSpendabilityVar = witness(cs.clone(), self)?;
        password_spendability_var.verify(&signing_hash, &spendability_address)?;

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
        let context_hash = variable(cs.clone(), &value.context_hash, mode)?;
        let secret = variable(cs.clone(), &value.password, mode)?;
        Ok(Self {
            operation,
            context_hash,
            secret,
        })
    }
}

impl PasswordSpendabilityVar {
    #[tracing::instrument(target = "gr1cs", skip_all)]
    pub fn verify(
        &self,
        signing_hash: &FrVar,
        spendability_address: &FrVar,
    ) -> Result<(), SynthesisError> {
        self.verify_signing_hash(signing_hash)?;
        self.verify_spendability_witnesses(spendability_address)?;

        Ok(())
    }

    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn verify_signing_hash(&self, signing_hash: &FrVar) -> Result<(), SynthesisError> {
        let operation_hash = self.operation.hash()?;
        let computed_signing_hash =
            poseidon2_compress_gadget(&[operation_hash, self.context_hash.clone()])?;
        computed_signing_hash.enforce_equal(signing_hash)
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
    use alloy_primitives::{Address, address};
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
        operation.inputs[0].note.policy.address = spendability_address;
        operation.inputs[0].note.policy.witness = witness;

        let circuit = signed_circuit(spendability_address, operation, secret);
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
        operation.inputs[0].note.policy.address = spendability_address;
        operation.inputs[0].note.policy.witness = witness;
        operation.inputs[1].note.policy.address = spendability_address;
        operation.inputs[1].note.policy.witness = witness;

        let circuit = signed_circuit(spendability_address, operation, secret);
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
        operation.inputs[0].note.policy.address = spendability_address;
        operation.inputs[0].note.policy.witness = witness;

        let circuit = signed_circuit(spendability_address, operation, invalid_secret);
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
        operation.inputs[0].note.policy.address = spendability_address;
        operation.inputs[0].note.policy.witness = witness;
        operation.inputs[1].note.policy.address = spendability_address;
        operation.inputs[1].note.policy.witness = other_witness;

        let circuit = signed_circuit(spendability_address, operation, secret);
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
    fn invalid_signing_hash() {
        let _guard = setup_constraint_tracing();
        let cs = ConstraintSystem::<Fr>::new_ref();

        let spendability_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);

        let mut operation = Operation::default();
        operation.inputs[0].note.policy.address = spendability_address;
        operation.inputs[0].note.policy.witness = witness;

        let mut circuit = signed_circuit(spendability_address, operation, secret);
        circuit.signing_hash += Fr::from(1);
        circuit.synthesize(&cs).unwrap();

        let failed = cs
            .which_is_unsatisfied()
            .unwrap()
            .expect("expected some unsatisfied constraints");
        assert!(
            failed.contains("verify_signing_hash"),
            "expected failure in verify_signing_hash, got:\n{failed}"
        );
    }

    /// Builds a circuit whose signing hash commits to `operation` and an
    /// arbitrary context hash.
    fn signed_circuit(
        spendability_address: Address,
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
        secret: Fr,
    ) -> PasswordSpendability {
        let context_hash = Fr::from(7);
        let signing_hash = poseidon2_compress(&[operation.hash(), context_hash]);

        PasswordSpendability::new(
            address_to_fr(spendability_address),
            signing_hash,
            operation,
            context_hash,
            secret,
        )
    }
}

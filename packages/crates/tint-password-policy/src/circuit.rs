use ark_bn254::Fr;
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, OptimizationGoal, SynthesisError,
};
use tint::{
    circuit::{
        FrVar,
        gadgets::operation::OperationVar,
        input,
        join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
        variable, witness,
    },
    operation::Operation,
};
use tint_poseidon2::poseidon2_compress_gadget;

/// Policy circuit for the "Password" rule, which proves knowledge of a
/// private key bound to a note's policy hash for a given operation's
/// signing hash.
#[derive(Clone, Default)]
pub struct PasswordPolicyCircuit {
    pub policy_address: Fr,
    pub signing_hash: Fr,

    // Witnessed values
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub context_hash: Fr,
    pub password: Fr,
}

pub struct PasswordPolicyCircuitVar {
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub context_hash: FrVar,
    pub secret: FrVar,
}

impl PasswordPolicyCircuit {
    pub fn new(
        policy_address: Fr,
        signing_hash: Fr,
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
        context_hash: Fr,
        secret: Fr,
    ) -> Self {
        Self {
            policy_address,
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
        let policy_address: FrVar = input(cs.clone(), &self.policy_address)?;
        let signing_hash: FrVar = input(cs.clone(), &self.signing_hash)?;

        // Witnessed values
        let password_policy_var: PasswordPolicyCircuitVar = witness(cs.clone(), self)?;
        password_policy_var.verify(&signing_hash, &policy_address)?;

        Ok(())
    }
}

impl ConstraintSynthesizer<Fr> for PasswordPolicyCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        self.synthesize(&cs)
    }
}

impl AllocVar<PasswordPolicyCircuit, Fr> for PasswordPolicyCircuitVar {
    fn new_variable<T: std::borrow::Borrow<PasswordPolicyCircuit>>(
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

impl PasswordPolicyCircuitVar {
    #[tracing::instrument(target = "gr1cs", skip_all)]
    pub fn verify(
        &self,
        signing_hash: &FrVar,
        policy_address: &FrVar,
    ) -> Result<(), SynthesisError> {
        self.verify_signing_hash(signing_hash)?;
        self.verify_policy_witnesses(policy_address)?;

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
    fn verify_policy_witnesses(&self, policy_address: &FrVar) -> Result<(), SynthesisError> {
        for input in &self.operation.inputs {
            let policy_addresses_eq = input.policy_address.is_eq(policy_address)?;
            let expected_witness = poseidon2_compress_gadget(core::array::from_ref(&self.secret))?;
            input
                .policy_witness
                .conditional_enforce_equal(&expected_witness, &policy_addresses_eq)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, address};
    use ark_relations::gr1cs::trace::{ConstraintLayer, TracingMode};
    use tint::fr::address_to_fr;
    use tint_poseidon2::poseidon2_compress;
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

        let policy_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);

        let mut operation = Operation::default();
        operation.inputs[0].note.policy.address = policy_address;
        operation.inputs[0].note.policy.witness = witness;

        let circuit = signed_circuit(policy_address, operation, secret);
        circuit.synthesize(&cs).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    #[test]
    fn multiple_inputs() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let policy_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);

        let mut operation = Operation::default();
        operation.inputs[0].note.policy.address = policy_address;
        operation.inputs[0].note.policy.witness = witness;
        operation.inputs[1].note.policy.address = policy_address;
        operation.inputs[1].note.policy.witness = witness;

        let circuit = signed_circuit(policy_address, operation, secret);
        circuit.synthesize(&cs).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    #[test]
    fn invalid_secret() {
        let _guard = setup_constraint_tracing();
        let cs = ConstraintSystem::<Fr>::new_ref();

        let policy_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);
        let invalid_secret = Fr::from(54321);

        let mut operation = Operation::default();
        operation.inputs[0].note.policy.address = policy_address;
        operation.inputs[0].note.policy.witness = witness;

        let circuit = signed_circuit(policy_address, operation, invalid_secret);
        circuit.synthesize(&cs).unwrap();

        let failed = cs
            .which_is_unsatisfied()
            .unwrap()
            .expect("expected some unsatisfied constraints");
        assert!(
            failed.contains("verify_policy_witnesses"),
            "expected failure in verify_policy_witnesses, got:\n{failed}"
        );
    }

    #[test]
    fn multiple_inputs_invalid_secret() {
        let _guard = setup_constraint_tracing();
        let cs = ConstraintSystem::<Fr>::new_ref();

        let policy_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);
        let other_witness = poseidon2_compress(&[Fr::from(54321)]);

        let mut operation = Operation::default();
        operation.inputs[0].note.policy.address = policy_address;
        operation.inputs[0].note.policy.witness = witness;
        operation.inputs[1].note.policy.address = policy_address;
        operation.inputs[1].note.policy.witness = other_witness;

        let circuit = signed_circuit(policy_address, operation, secret);
        circuit.synthesize(&cs).unwrap();

        let failed = cs
            .which_is_unsatisfied()
            .unwrap()
            .expect("expected some unsatisfied constraints");
        assert!(
            failed.contains("verify_policy_witnesses"),
            "expected failure in verify_policy_witnesses, got:\n{failed}"
        );
    }

    #[test]
    fn invalid_signing_hash() {
        let _guard = setup_constraint_tracing();
        let cs = ConstraintSystem::<Fr>::new_ref();

        let policy_address = address!("0x1234567890abcdef1234567890abcdef12345678");
        let secret = Fr::from(12345);
        let witness = poseidon2_compress(&[secret]);

        let mut operation = Operation::default();
        operation.inputs[0].note.policy.address = policy_address;
        operation.inputs[0].note.policy.witness = witness;

        let mut circuit = signed_circuit(policy_address, operation, secret);
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
        policy_address: Address,
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
        secret: Fr,
    ) -> PasswordPolicyCircuit {
        let context_hash = Fr::from(7);
        let signing_hash = poseidon2_compress(&[operation.hash(), context_hash]);

        PasswordPolicyCircuit::new(
            address_to_fr(policy_address),
            signing_hash,
            operation,
            context_hash,
            secret,
        )
    }
}

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
    operation::OperationWitness,
};
use tint_poseidon2::poseidon2_compress_gadget;

/// Bit width of unlock times and of the gap between an unlock time and its bound.
const TIME_BITS: usize = 64;

/// Policy circuit for the "Timelock" rule, which proves a note's committed
/// unlock time is at most a public `bound` for a given operation's signing
/// hash.
#[derive(Clone, Default)]
pub struct TimelockPolicyCircuit {
    pub policy_address: Fr,
    pub signing_hash: Fr,
    pub bound: Fr,

    // Witnessed values
    pub operation: OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub context_hash: Fr,
    pub unlock_time: Fr,
    pub salt: Fr,
}

pub struct TimelockPolicyCircuitVar {
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub context_hash: FrVar,
    pub unlock_time: FrVar,
    pub salt: FrVar,
}

impl TimelockPolicyCircuit {
    pub fn new(
        policy_address: Fr,
        signing_hash: Fr,
        bound: Fr,
        operation: OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
        context_hash: Fr,
        unlock_time: Fr,
        salt: Fr,
    ) -> Self {
        Self {
            policy_address,
            signing_hash,
            bound,
            operation,
            context_hash,
            unlock_time,
            salt,
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
        let bound: FrVar = input(cs.clone(), &self.bound)?;

        // Witnessed values
        let timelock_policy_var: TimelockPolicyCircuitVar = witness(cs.clone(), self)?;
        timelock_policy_var.verify(&signing_hash, &policy_address, &bound)?;

        Ok(())
    }
}

impl ConstraintSynthesizer<Fr> for TimelockPolicyCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        self.synthesize(&cs)
    }
}

impl AllocVar<TimelockPolicyCircuit, Fr> for TimelockPolicyCircuitVar {
    fn new_variable<T: std::borrow::Borrow<TimelockPolicyCircuit>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let operation = variable(cs.clone(), &value.operation, mode)?;
        let context_hash = variable(cs.clone(), &value.context_hash, mode)?;
        let unlock_time = variable(cs.clone(), &value.unlock_time, mode)?;
        let salt = variable(cs.clone(), &value.salt, mode)?;
        Ok(Self {
            operation,
            context_hash,
            unlock_time,
            salt,
        })
    }
}

impl TimelockPolicyCircuitVar {
    #[tracing::instrument(target = "gr1cs", skip_all)]
    pub fn verify(
        &self,
        signing_hash: &FrVar,
        policy_address: &FrVar,
        bound: &FrVar,
    ) -> Result<(), SynthesisError> {
        self.verify_signing_hash(signing_hash)?;
        self.verify_policy_witnesses(policy_address)?;
        self.verify_unlocked(bound)?;

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
        let expected_witness =
            poseidon2_compress_gadget(&[self.unlock_time.clone(), self.salt.clone()])?;
        for input in &self.operation.inputs {
            let policy_addresses_eq = input.policy_address.is_eq(policy_address)?;
            input
                .policy_witness
                .conditional_enforce_equal(&expected_witness, &policy_addresses_eq)?;
        }
        Ok(())
    }

    /// Enforces `unlock_time <= bound` for an unlock time of `TIME_BITS` bits.
    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn verify_unlocked(&self, bound: &FrVar) -> Result<(), SynthesisError> {
        let _ = self.unlock_time.to_bits_le_with_top_bits_zero(TIME_BITS)?;
        let _ = (bound - &self.unlock_time).to_bits_le_with_top_bits_zero(TIME_BITS)?;
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

    const POLICY_ADDRESS: Address = address!("0x1234567890abcdef1234567890abcdef12345678");
    const UNLOCK_TIME: u64 = 1_000;
    const SALT: u64 = 42;

    fn setup_constraint_tracing() -> tracing::subscriber::DefaultGuard {
        let mut layer = ConstraintLayer::default();
        layer.mode = TracingMode::OnlyConstraints;
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        tracing::subscriber::set_default(subscriber)
    }

    #[test]
    fn bound_at_unlock_time() {
        let circuit = signed_circuit(locked_operation(1), Fr::from(UNLOCK_TIME));
        assert_satisfied(&circuit);
    }

    #[test]
    fn bound_after_unlock_time() {
        let circuit = signed_circuit(locked_operation(1), Fr::from(UNLOCK_TIME + 500));
        assert_satisfied(&circuit);
    }

    #[test]
    fn multiple_inputs() {
        let circuit = signed_circuit(locked_operation(2), Fr::from(UNLOCK_TIME));
        assert_satisfied(&circuit);
    }

    #[test]
    fn bound_before_unlock_time() {
        let circuit = signed_circuit(locked_operation(1), Fr::from(UNLOCK_TIME - 1));
        assert_unsatisfied_in(&circuit, "verify_unlocked");
    }

    #[test]
    fn unlock_time_out_of_range() {
        // A "negative" unlock time would otherwise be below every bound.
        let unlock_time = -Fr::from(1u64);
        let mut operation = OperationWitness::default();
        operation.inputs[0].note.policy.address = POLICY_ADDRESS;
        operation.inputs[0].note.policy.witness =
            poseidon2_compress(&[unlock_time, Fr::from(SALT)]);

        let mut circuit = signed_circuit(operation, Fr::from(0u64));
        circuit.unlock_time = unlock_time;
        assert_unsatisfied_in(&circuit, "verify_unlocked");
    }

    #[test]
    fn invalid_salt() {
        let mut circuit = signed_circuit(locked_operation(1), Fr::from(UNLOCK_TIME));
        circuit.salt += Fr::from(1u64);
        assert_unsatisfied_in(&circuit, "verify_policy_witnesses");
    }

    #[test]
    fn earlier_unlock_time() {
        let mut circuit = signed_circuit(locked_operation(1), Fr::from(UNLOCK_TIME));
        circuit.unlock_time = Fr::from(UNLOCK_TIME - 1);
        assert_unsatisfied_in(&circuit, "verify_policy_witnesses");
    }

    #[test]
    fn invalid_signing_hash() {
        let mut circuit = signed_circuit(locked_operation(1), Fr::from(UNLOCK_TIME));
        circuit.signing_hash += Fr::from(1u64);
        assert_unsatisfied_in(&circuit, "verify_signing_hash");
    }

    fn assert_satisfied(circuit: &TimelockPolicyCircuit) {
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.synthesize(&cs).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    fn assert_unsatisfied_in(circuit: &TimelockPolicyCircuit, scope: &str) {
        let _guard = setup_constraint_tracing();
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.synthesize(&cs).unwrap();

        let failed = cs
            .which_is_unsatisfied()
            .unwrap()
            .expect("expected some unsatisfied constraints");
        assert!(
            failed.contains(scope),
            "expected failure in {scope}, got:\n{failed}"
        );
    }

    /// An operation whose first `n` inputs are locked until `UNLOCK_TIME`.
    fn locked_operation(n: usize) -> OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
        let witness = poseidon2_compress(&[Fr::from(UNLOCK_TIME), Fr::from(SALT)]);
        let mut operation = OperationWitness::default();
        for input in &mut operation.inputs[..n] {
            input.note.policy.address = POLICY_ADDRESS;
            input.note.policy.witness = witness;
        }
        operation
    }

    /// Builds a circuit whose signing hash commits to `operation` and an
    /// arbitrary context hash.
    fn signed_circuit(
        operation: OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
        bound: Fr,
    ) -> TimelockPolicyCircuit {
        let context_hash = Fr::from(7);
        let signing_hash = poseidon2_compress(&[operation.hash(), context_hash]);

        TimelockPolicyCircuit::new(
            address_to_fr(POLICY_ADDRESS),
            signing_hash,
            bound,
            operation,
            context_hash,
            Fr::from(UNLOCK_TIME),
            Fr::from(SALT),
        )
    }
}

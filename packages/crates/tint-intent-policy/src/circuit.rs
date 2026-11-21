use std::cmp::Ordering;

use ark_bn254::Fr;
use ark_r1cs_std::{
    alloc::AllocVar,
    eq::EqGadget,
    fields::FieldVar,
    prelude::{AllocationMode, Boolean},
};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, Namespace, OptimizationGoal,
    SynthesisError,
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

/// Policy circuit for intents: authorizes the input at `slot` if the operation
/// fills the intent before `deadline <= expiry`, or the prover knows its
/// cancel key.
#[derive(Clone, Default)]
pub struct IntentPolicyCircuit {
    pub policy_address: Fr,
    pub signing_hash: Fr,
    pub slot: Fr,
    pub deadline: Fr,

    // Witnessed values
    pub operation: OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub context_hash: Fr,
    pub want_hash: Fr,
    pub expiry: Fr,
    pub cancel_hash: Fr,
    /// Zero when filling.
    pub cancel_key: Fr,
}

struct IntentPolicyCircuitVar {
    operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    context_hash: FrVar,
    want_hash: FrVar,
    expiry: FrVar,
    cancel_hash: FrVar,
    cancel_key: FrVar,
}

/// The circuit's public inputs, in allocation order.
struct PublicInputs {
    policy_address: FrVar,
    signing_hash: FrVar,
    slot: FrVar,
    deadline: FrVar,
}

impl IntentPolicyCircuit {
    pub fn synthesize_public_inputs(&self) -> Result<Vec<Fr>, SynthesisError> {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);

        self.synthesize(&cs)?;
        cs.finalize();

        // `instance_assignment()` leads with the implicit constant-1 term.
        Ok(cs.instance_assignment()?[1..].to_vec())
    }

    fn synthesize(&self, cs: &ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let public = PublicInputs {
            policy_address: input(cs.clone(), &self.policy_address)?,
            signing_hash: input(cs.clone(), &self.signing_hash)?,
            slot: input(cs.clone(), &self.slot)?,
            deadline: input(cs.clone(), &self.deadline)?,
        };

        let var: IntentPolicyCircuitVar = witness(cs.clone(), self)?;
        var.verify(&public)
    }
}

impl ConstraintSynthesizer<Fr> for IntentPolicyCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        self.synthesize(&cs)
    }
}

impl AllocVar<IntentPolicyCircuit, Fr> for IntentPolicyCircuitVar {
    fn new_variable<T: std::borrow::Borrow<IntentPolicyCircuit>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        Ok(Self {
            operation: variable(cs.clone(), &value.operation, mode)?,
            context_hash: variable(cs.clone(), &value.context_hash, mode)?,
            want_hash: variable(cs.clone(), &value.want_hash, mode)?,
            expiry: variable(cs.clone(), &value.expiry, mode)?,
            cancel_hash: variable(cs.clone(), &value.cancel_hash, mode)?,
            cancel_key: variable(cs.clone(), &value.cancel_key, mode)?,
        })
    }
}

impl IntentPolicyCircuitVar {
    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn verify(&self, public: &PublicInputs) -> Result<(), SynthesisError> {
        self.verify_signing_hash(&public.signing_hash)?;
        self.verify_slot(&public.policy_address, &public.slot)?;

        let fill = self.is_fill(&public.deadline)?;
        let cancel = self.is_cancel()?;
        (fill | cancel).enforce_equal(&Boolean::TRUE)
    }

    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn verify_signing_hash(&self, signing_hash: &FrVar) -> Result<(), SynthesisError> {
        let operation_hash = self.operation.hash()?;
        let computed = poseidon2_compress_gadget(&[operation_hash, self.context_hash.clone()])?;
        computed.enforce_equal(signing_hash)
    }

    /// Verifies the input at `slot` is bound to this policy and these terms.
    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn verify_slot(&self, policy_address: &FrVar, slot: &FrVar) -> Result<(), SynthesisError> {
        let policy_witness = poseidon2_compress_gadget(&[
            self.want_hash.clone(),
            self.expiry.clone(),
            self.cancel_hash.clone(),
        ])?;

        let mut selected = Vec::with_capacity(N_INPUTS);
        for (i, input) in self.operation.inputs.iter().enumerate() {
            let is_slot = slot.is_eq(&FrVar::constant(Fr::from(i as u64)))?;
            input
                .policy_address
                .conditional_enforce_equal(policy_address, &is_slot)?;
            input
                .policy_witness
                .conditional_enforce_equal(&policy_witness, &is_slot)?;
            selected.push(is_slot);
        }
        Boolean::kary_or(&selected)?.enforce_equal(&Boolean::TRUE)
    }

    /// Whether the operation creates the wanted note, before `deadline <= expiry`.
    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn is_fill(&self, deadline: &FrVar) -> Result<Boolean<Fr>, SynthesisError> {
        let outputs = self.operation.output_commitment_hashes()?;
        let created = outputs
            .iter()
            .map(|hash| hash.is_eq(&self.want_hash))
            .collect::<Result<Vec<_>, _>>()?;
        let created = Boolean::kary_or(&created)?;

        let wants_note = !self.want_hash.is_eq(&FrVar::zero())?;
        let in_time = deadline.is_cmp(&self.expiry, Ordering::Less, true)?;
        Ok(created & wants_note & in_time)
    }

    /// Whether the prover knows the cancel key.
    #[tracing::instrument(target = "gr1cs", skip_all)]
    fn is_cancel(&self) -> Result<Boolean<Fr>, SynthesisError> {
        let token = poseidon2_compress_gadget(core::array::from_ref(&self.cancel_key))?;
        poseidon2_compress_gadget(&[token])?.is_eq(&self.cancel_hash)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, B256};
    use ark_ff::AdditiveGroup;
    use ark_relations::gr1cs::trace::{ConstraintLayer, TracingMode};
    use ark_std::rand::{SeedableRng, rngs::StdRng};
    use tint::{
        account::Account,
        fr::address_to_fr,
        note::{asset::AssetId, owned::SpendableNote},
        policy::Policy,
    };
    use tint_poseidon2::poseidon2_compress;
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;
    use crate::intent::{CancelKey, Intent};

    const POLICY: Address = Address::repeat_byte(3);
    const EXPIRY: u64 = 1_000;

    #[test]
    fn fill() {
        let (intent, _) = intent();
        let circuit = circuit(&intent, filled(&intent), 0, EXPIRY, Fr::ZERO);
        assert_satisfied(&circuit);
    }

    #[test]
    fn fill_before_expiry() {
        let (intent, _) = intent();
        let circuit = circuit(&intent, filled(&intent), 0, EXPIRY - 1, Fr::ZERO);
        assert_satisfied(&circuit);
    }

    #[test]
    fn fill_after_expiry() {
        let (intent, _) = intent();
        let circuit = circuit(&intent, filled(&intent), 0, EXPIRY + 1, Fr::ZERO);
        assert_unsatisfied(&circuit, "verify");
    }

    #[test]
    fn missing_want() {
        let (intent, _) = intent();
        let mut operation = filled(&intent);
        operation.outputs[0].amount -= 1;

        let circuit = circuit(&intent, operation, 0, EXPIRY, Fr::ZERO);
        assert_unsatisfied(&circuit, "verify");
    }

    #[test]
    fn cancel() {
        let (intent, key) = intent();
        let circuit = circuit(&intent, unfilled(&intent), 0, u64::MAX, key.0);
        assert_satisfied(&circuit);
    }

    #[test]
    fn wrong_cancel_key() {
        let (intent, key) = intent();
        let circuit = circuit(&intent, unfilled(&intent), 0, u64::MAX, key.token());
        assert_unsatisfied(&circuit, "verify");
    }

    #[test]
    fn wrong_slot() {
        let (intent, _) = intent();
        let circuit = circuit(&intent, filled(&intent), 1, EXPIRY, Fr::ZERO);
        assert_unsatisfied(&circuit, "verify_slot");
    }

    #[test]
    fn out_of_range_slot() {
        let (intent, _) = intent();
        let circuit = circuit(&intent, filled(&intent), N_INPUTS as u64, EXPIRY, Fr::ZERO);
        assert_unsatisfied(&circuit, "verify_slot");
    }

    #[test]
    fn wrong_terms() {
        let (intent, _) = intent();
        let mut circuit = circuit(&intent, filled(&intent), 0, EXPIRY, Fr::ZERO);
        circuit.expiry = Fr::from(EXPIRY + 1);
        assert_unsatisfied(&circuit, "verify_slot");
    }

    #[test]
    fn wrong_signing_hash() {
        let (intent, _) = intent();
        let mut circuit = circuit(&intent, filled(&intent), 0, EXPIRY, Fr::ZERO);
        circuit.signing_hash += Fr::from(1);
        assert_unsatisfied(&circuit, "verify_signing_hash");
    }

    fn assert_satisfied(circuit: &IntentPolicyCircuit) {
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.synthesize(&cs).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    fn assert_unsatisfied(circuit: &IntentPolicyCircuit, expected: &str) {
        let mut layer = ConstraintLayer::default();
        layer.mode = TracingMode::OnlyConstraints;
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.synthesize(&cs).unwrap();
        let failed = cs
            .which_is_unsatisfied()
            .unwrap()
            .expect("expected some unsatisfied constraints");
        assert!(
            failed.contains(expected),
            "expected failure in {expected}, got:\n{failed}"
        );
    }

    fn intent() -> (Intent, CancelKey) {
        let receiver = Account::from_seed(&[1; 32], Policy::NONE).pub_account();
        let a = AssetId::from(Address::repeat_byte(1));
        let b = AssetId::from(Address::repeat_byte(2));
        let mut rng = StdRng::seed_from_u64(1);
        Intent::new(POLICY, (a, 100), (b, 50), &receiver, EXPIRY, &mut rng).unwrap()
    }

    /// An operation spending `intent`'s offer in slot 0 and creating its want.
    fn filled(intent: &Intent) -> OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
        let mut operation = unfilled(intent);
        operation.outputs[0] = intent.want;
        operation
    }

    /// An operation spending `intent`'s offer in slot 0 back to its owner.
    fn unfilled(intent: &Intent) -> OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
        let mut operation = OperationWitness::default();
        operation.inputs[0] = SpendableNote {
            note: intent.note(),
            policy_input: alloy_primitives::Bytes::default(),
        };
        operation.outputs[0] = intent.offer;
        operation.outputs[0].random = B256::repeat_byte(9);
        operation
    }

    fn circuit(
        intent: &Intent,
        operation: OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
        slot: u64,
        deadline: u64,
        cancel_key: Fr,
    ) -> IntentPolicyCircuit {
        let context_hash = Fr::from(7);
        IntentPolicyCircuit {
            policy_address: address_to_fr(POLICY),
            signing_hash: poseidon2_compress(&[operation.hash(), context_hash]),
            slot: Fr::from(slot),
            deadline: Fr::from(deadline),
            operation,
            context_hash,
            want_hash: intent.want.hash(),
            expiry: Fr::from(intent.expiry),
            cancel_hash: intent.cancel_hash,
            cancel_key,
        }
    }
}

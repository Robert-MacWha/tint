use alloy_primitives::{Address, Bytes};
use alloy_sol_types::SolValue;
use ark_bn254::{Bn254, Fr};
use ark_ff::AdditiveGroup;
use ark_groth16::Groth16;
use ark_relations::gr1cs::SynthesisError;
use ark_snark::SNARK;
use rand_core::OsRng;
use tint::{
    abis::tint::ProofLib,
    fr::address_to_fr,
    operation::{Operation, OperationError},
};
use tint_groth16::{
    artifacts::Artifacts,
    prove::{placeholder, prove_with_matrices},
};
use tracing::info;

use crate::{
    circuit::IntentPolicyCircuit,
    intent::{CancelKey, Intent},
};

/// Authorizes spending intent notes bound to the [`IntentPolicyCircuit`].
#[derive(Clone)]
pub struct IntentPolicy {
    contract_address: Address,
    artifacts: Artifacts<Bn254>,
}

#[derive(Debug, thiserror::Error)]
pub enum IntentPolicyError {
    #[error("intent is bound to policy {0}, not this deployment")]
    WrongPolicy(Address),
    #[error("no input of the operation spends the intent")]
    UnusedIntent,
    #[error("operation error: {0}")]
    Operation(#[from] OperationError),
    #[error("proving error: {0}")]
    Synthesis(#[from] SynthesisError),
    #[error("generated proof is invalid")]
    InvalidProof,
}

impl IntentPolicy {
    #[must_use]
    pub fn new(contract_address: Address, artifacts: Artifacts<Bn254>) -> Self {
        Self {
            contract_address,
            artifacts,
        }
    }

    /// The `IntentPolicy` deployment this prover targets.
    #[must_use]
    pub fn address(&self) -> Address {
        self.contract_address
    }

    /// Authorizes `op` to spend `intent`'s offer, which `op` must fill.
    ///
    /// # Errors
    ///
    /// Errors if `op` doesn't spend the intent, or proving fails.
    pub fn authorize_fill(
        &self,
        op: &mut Operation,
        intent: &Intent,
    ) -> Result<(), IntentPolicyError> {
        self.authorize(op, intent, intent.expiry, Fr::ZERO)
    }

    /// Authorizes `op` to spend `intent`'s offer however its owner likes.
    ///
    /// # Errors
    ///
    /// Errors if `op` doesn't spend the intent, or proving fails.
    pub fn authorize_cancel(
        &self,
        op: &mut Operation,
        intent: &Intent,
        key: &CancelKey,
    ) -> Result<(), IntentPolicyError> {
        self.authorize(op, intent, u64::MAX, key.0)
    }

    /// Assigns a placeholder authorization to `intent`'s input of `op`.
    ///
    /// Placeholder authorizations are expected to revert, but consume an equivalent
    /// amount of gas to a valid authorization. Used when simulating operations.
    ///
    /// # Errors
    ///
    /// Errors if `op` doesn't spend the intent.
    pub fn authorize_placeholder(
        &self,
        op: &mut Operation,
        intent: &Intent,
    ) -> Result<(), OperationError> {
        let proof = ProofLib::Proof::from(placeholder::<Bn254>());
        op.authorize(intent.policy(), policy_input(proof, u64::MAX))
    }

    fn authorize(
        &self,
        op: &mut Operation,
        intent: &Intent,
        deadline: u64,
        cancel_key: Fr,
    ) -> Result<(), IntentPolicyError> {
        if intent.policy_address != self.contract_address {
            return Err(IntentPolicyError::WrongPolicy(intent.policy_address));
        }

        let policy = intent.policy();
        let witness = op.witness();
        let slot = witness
            .inputs
            .iter()
            .position(|i| i.is_live() && i.note.policy == policy)
            .ok_or(IntentPolicyError::UnusedIntent)?;

        let circuit = IntentPolicyCircuit {
            policy_address: address_to_fr(self.contract_address),
            signing_hash: witness.signing_hash(),
            slot: Fr::from(slot as u64),
            deadline: Fr::from(deadline),
            operation: witness.clone(),
            context_hash: witness.context_hash(),
            want_hash: intent.want.hash(),
            expiry: Fr::from(intent.expiry),
            cancel_hash: intent.cancel_hash,
            cancel_key,
        };

        info!("Proving intent policy...");
        let (public_inputs, proof) = prove_with_matrices(
            &self.artifacts.matrices,
            &self.artifacts.pk,
            &circuit,
            &mut OsRng,
        )?;

        // Smoke-test the proof locally
        if !Groth16::<Bn254>::verify(&self.artifacts.vk, &public_inputs, &proof)? {
            return Err(IntentPolicyError::InvalidProof);
        }
        info!("Intent policy proof verified");

        op.authorize(policy, policy_input(proof.into(), deadline))?;
        Ok(())
    }
}

impl std::fmt::Debug for IntentPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntentPolicy")
            .field("contract_address", &self.contract_address)
            .finish_non_exhaustive()
    }
}

/// `abi.encode(proof, deadline)`, as `IntentPolicy.sol` decodes it.
fn policy_input(proof: ProofLib::Proof, deadline: u64) -> Bytes {
    (proof, deadline).abi_encode_params().into()
}

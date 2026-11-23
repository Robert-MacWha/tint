use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolValue;
use ark_bn254::{Bn254, Fr};
use ark_groth16::Groth16;
use ark_relations::gr1cs::SynthesisError;
use ark_snark::SNARK;
use rand_core::OsRng;
use tint::{
    abis::tint::ProofLib,
    fr::address_to_fr,
    operation::{Operation, OperationError},
    policy::Policy,
};
use tint_groth16::{
    artifacts::Artifacts,
    prove::{placeholder, prove_with_matrices},
};
use tint_poseidon2::poseidon2_compress;
use tracing::info;

use crate::circuit::TimelockPolicyCircuit;

/// Authorizes spending notes bound to the [`TimelockPolicyCircuit`].
#[derive(Clone)]
pub struct TimelockPolicy {
    contract_address: Address,
    unlock_time: u64,
    salt: Fr,
    artifacts: Artifacts<Bn254>,
}

#[derive(Debug, thiserror::Error)]
pub enum TimelockPolicyError {
    #[error("bound {bound} is before the unlock time")]
    Locked { bound: u64 },
    #[error("operation error: {0}")]
    Operation(#[from] OperationError),
    #[error("proving error: {0}")]
    Synthesis(#[from] SynthesisError),
    #[error("generated proof is invalid")]
    InvalidProof,
}

impl TimelockPolicy {
    /// Locks notes until the `unlock_time` unix timestamp, hidden by `salt`.
    #[must_use]
    pub fn new(
        contract_address: Address,
        unlock_time: u64,
        salt: Fr,
        artifacts: Artifacts<Bn254>,
    ) -> Self {
        Self {
            contract_address,
            unlock_time,
            salt,
            artifacts,
        }
    }

    /// The policy notes locked until this unlock time are bound to.
    #[must_use]
    pub fn policy(&self) -> Policy {
        let witness = poseidon2_compress(&[Fr::from(self.unlock_time), self.salt]);
        Policy::new(self.contract_address, witness)
    }

    /// Authorizes every input of `op` bound to this policy, revealing `bound`.
    ///
    /// The operation is only valid once `bound <= block.timestamp`.
    ///
    /// # Errors
    ///
    /// Errors if `bound` is before the unlock time, proving fails, or no input
    /// of `op` is bound to this policy.
    pub fn authorize(&self, op: &mut Operation, bound: u64) -> Result<(), TimelockPolicyError> {
        let input = self.prove(op, bound)?;
        op.authorize(self.policy(), input)?;
        Ok(())
    }

    /// Assigns placeholder authorizations to every input of `op` bound to this policy.
    ///
    /// Placeholder authorizations are expected to revert, but consume an equivalent
    /// amount of gas to a valid authorization. Used when simulating operations.
    ///
    /// # Errors
    ///
    /// Errors if no input of `op` is bound to this policy.
    pub fn authorize_placeholder(
        &self,
        op: &mut Operation,
        bound: u64,
    ) -> Result<(), OperationError> {
        let input = encode_input(placeholder::<Bn254>(), bound);
        op.authorize(self.policy(), input)
    }

    /// Proves the unlock time is at most `bound` over `op`'s signing hash. The
    /// returned policy input authorizes every input of `op` bound to this policy.
    ///
    /// # Errors
    ///
    /// Errors if `bound` is before the unlock time, or proving fails.
    pub fn prove(&self, op: &Operation, bound: u64) -> Result<Bytes, TimelockPolicyError> {
        if bound < self.unlock_time {
            return Err(TimelockPolicyError::Locked { bound });
        }

        let witness = op.witness();
        let circuit = TimelockPolicyCircuit::new(
            address_to_fr(self.contract_address),
            witness.signing_hash(),
            Fr::from(bound),
            witness.clone(),
            witness.context_hash(),
            Fr::from(self.unlock_time),
            self.salt,
        );

        info!("Proving policy...");
        let (public_inputs, proof) = prove_with_matrices(
            &self.artifacts.matrices,
            &self.artifacts.pk,
            &circuit,
            &mut OsRng,
        )?;

        // Smoke-test the proof locally
        if !Groth16::<Bn254>::verify(&self.artifacts.vk, &public_inputs, &proof)? {
            return Err(TimelockPolicyError::InvalidProof);
        }
        info!("Policy proof verified");

        Ok(encode_input(proof, bound))
    }
}

impl std::fmt::Debug for TimelockPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimelockPolicy")
            .field("contract_address", &self.contract_address)
            .field("unlock_time", &"[REDACTED]")
            .field("salt", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Encodes a policy input as `TimelockPolicy.sol` decodes it.
fn encode_input(proof: ark_groth16::Proof<Bn254>, bound: u64) -> Bytes {
    (ProofLib::Proof::from(proof), U256::from(bound))
        .abi_encode_params()
        .into()
}

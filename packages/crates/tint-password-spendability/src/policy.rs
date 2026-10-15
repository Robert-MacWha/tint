use alloy_primitives::{Address, Bytes};
use alloy_sol_types::SolValue;
use ark_bn254::{Bn254, Fr};
use ark_groth16::Groth16;
use ark_relations::gr1cs::SynthesisError;
use ark_snark::SNARK;
use rand_core::OsRng;
use tint::{
    abis::tint::ProofLib, fr::address_to_fr, operation::UnauthorizedOperation, policy::Policy,
};
use tint_groth16::{artifacts::Artifacts, prove::prove_with_matrices};
use tint_poseidon2::poseidon2_compress;
use tracing::info;

use crate::circuit::PasswordSpendability;

/// Authorizes spending notes bound to the [`PasswordSpendability`] circuit.
#[derive(Clone)]
pub struct PasswordPolicy {
    contract_address: Address,
    secret: Fr,
    artifacts: Artifacts<Bn254>,
}

#[derive(Debug, thiserror::Error)]
pub enum PasswordPolicyError {
    #[error("proving error: {0}")]
    Synthesis(#[from] SynthesisError),
    #[error("generated proof is invalid")]
    InvalidProof,
}

impl PasswordPolicy {
    #[must_use]
    pub fn new(contract_address: Address, secret: Fr, artifacts: Artifacts<Bn254>) -> Self {
        Self {
            contract_address,
            secret,
            artifacts,
        }
    }

    /// The policy notes for this password are bound to.
    #[must_use]
    pub fn policy(&self) -> Policy {
        Policy::new(self.contract_address, poseidon2_compress(&[self.secret]))
    }

    /// Proves knowledge of the password over `op`'s signing hash. The returned
    /// policy input authorizes every input of `op` bound to this policy.
    ///
    /// # Errors
    ///
    /// Errors if proving fails.
    pub fn authorize(&self, op: &UnauthorizedOperation) -> Result<Bytes, PasswordPolicyError> {
        let operation = op.operation();
        let circuit = PasswordSpendability::new(
            address_to_fr(self.contract_address),
            operation.signing_hash(),
            operation.clone(),
            operation.context_hash(),
            self.secret,
        );

        info!("Proving spendability...");
        let (public_inputs, proof) = prove_with_matrices(
            &self.artifacts.matrices,
            &self.artifacts.pk,
            &circuit,
            &mut OsRng,
        )?;

        // Smoke-test the proof locally
        if !Groth16::<Bn254>::verify(&self.artifacts.vk, &public_inputs, &proof)? {
            return Err(PasswordPolicyError::InvalidProof);
        }
        info!("Spendability proof verified");

        Ok(ProofLib::Proof::from(proof).abi_encode().into())
    }
}

impl std::fmt::Debug for PasswordPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordPolicy")
            .field("contract_address", &self.contract_address)
            .finish_non_exhaustive()
    }
}

use alloy_primitives::{Address, Bytes};
use alloy_sol_types::SolValue;
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_snark::SNARK;
use rand_core::OsRng;
use tint::{
    account::spending::{SpendingAccount, SpendingAccountError},
    circuit::{
        join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
        matrices::{Matrices, prove_with_matrices},
        poseidon2::poseidon2_compress,
    },
    fr::address_to_fr,
    note::commitment::{NullifiableCommitment, SpendableCommitment},
    operation::Operation,
};
use tracing::info;

use crate::{abis::ProofLib, circuit::PasswordSpendability};

/// A [`SpendingAccount`] for notes using the [`PasswordSpendability`] circuit.
#[derive(Clone)]
pub struct PasswordSpendingAccount {
    contract_address: Address,
    secret: Fr,
    matrices: Matrices<Fr>,
    pk: ProvingKey<Bn254>,
    vk: VerifyingKey<Bn254>,
}

impl PasswordSpendingAccount {
    pub fn new(
        contract_address: Address,
        secret: impl FnOnce() -> Result<Fr, Box<dyn std::error::Error + Send + Sync>>,
        matrices: Matrices<Fr>,
        pk: ProvingKey<Bn254>,
        vk: VerifyingKey<Bn254>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            contract_address,
            secret: secret()?,
            matrices,
            pk,
            vk,
        })
    }

    pub fn new_const(
        contract_address: Address,
        secret: Fr,
        matrices: Matrices<Fr>,
        pk: ProvingKey<Bn254>,
        vk: VerifyingKey<Bn254>,
    ) -> Self {
        Self {
            contract_address,
            secret,
            matrices,
            pk,
            vk,
        }
    }
}

impl std::fmt::Debug for PasswordSpendingAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordSpendingAccount")
            .field("contract_address", &self.contract_address)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl SpendingAccount for PasswordSpendingAccount {
    fn spendability_address(&self) -> Address {
        self.contract_address
    }

    fn spendability_witness(&self) -> Fr {
        poseidon2_compress(&[self.secret])
    }

    /// Proves knowledge of `secret`
    async fn into_spendable(
        &self,
        base: NullifiableCommitment,
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    ) -> Result<SpendableCommitment, SpendingAccountError> {
        let operation_hash = operation.hash();
        let secret = self.secret;

        let circuit = PasswordSpendability::new(
            address_to_fr(self.contract_address),
            operation_hash,
            operation,
            secret,
        );

        info!("Proving spendability...");
        let (public_inputs, proof) =
            prove_with_matrices(&self.matrices, &self.pk, circuit, &mut OsRng)
                .map_err(SpendingAccountError::new)?;

        // Smoke-test the proof locally
        match Groth16::<Bn254>::verify(&self.vk, &public_inputs, &proof) {
            Ok(true) => {}
            Ok(false) => return Err(SpendingAccountError::from_str("invalid proof")),
            Err(e) => return Err(SpendingAccountError::new(e)),
        }
        info!("Spendability proof verified");

        let spendability_input = ProofLib::Proof::from(proof).abi_encode();
        Ok(SpendableCommitment::new(
            base.inner.asset,
            base.inner.amount,
            base.nullifier_key,
            self.spendability_address(),
            self.spendability_witness(),
            Bytes::from(spendability_input),
            base.inner.random,
        ))
    }
}

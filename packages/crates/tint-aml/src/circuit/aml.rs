use ark_bn254::Fr;
use ark_r1cs_std::eq::EqGadget;
use ark_relations::gr1cs::SynthesisError;
use tint::circuit::FrVar;
use tint::circuit::join_split::{K, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, TREE_DEPTH};
use tint::circuit::merkle_tree::InclusionProofVar;
use tint::circuit::operation::OperationVar;
use tint::indexer::merkle_tree::InclusionProof;
use tint::operation::Operation;

pub struct AML {
    // Public inputs
    pub new_root: Fr,
    pub expected_operation_hash: Fr,

    // Witnessed values
    pub commitment_inclusion_proofs: [InclusionProof<TREE_DEPTH, K>; N_INPUTS],
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct AMLVar {
    pub commitment_inclusion_proofs: [InclusionProofVar<TREE_DEPTH, K>; N_INPUTS],
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct AMLResult {
    pub nullifiers: [Fr; N_INPUTS],
    pub output_commitment_hashes: [Fr; N_OUTPUTS],
    pub withdrawal_amounts: [Fr; N_WITHDRAWALS],
    pub withdrawal_assets: [Fr; N_WITHDRAWALS],
}

pub struct AMLResultVar {
    pub nullifiers: [FrVar; N_INPUTS],
    pub output_commitment_hashes: [FrVar; N_OUTPUTS],
    pub withdrawal_amounts: [FrVar; N_WITHDRAWALS],
    pub withdrawal_assets: [FrVar; N_WITHDRAWALS],
}

impl AML {
    pub fn new(
        new_root: Fr,
        expected_operation_hash: Fr,
        commitment_inclusion_proofs: [InclusionProof<TREE_DEPTH, K>; N_INPUTS],
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    ) -> Self {
        Self {
            new_root,
            expected_operation_hash,
            commitment_inclusion_proofs,
            operation,
        }
    }
}

impl AMLVar {
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify(
        &self,
        new_root: FrVar,
        expected_operation_hash: FrVar,
    ) -> Result<AMLResultVar, SynthesisError> {
        let operation_hash = self.operation.hash()?;
        operation_hash.enforce_equal(&expected_operation_hash)?;

        todo!()

        // // Verify the inclusion proofs for the input commitments. Skipped for
        // // zero-valued leaves (padding).
        // for proof in &self.commitment_inclusion_proofs {
        //     let implied_root = proof.root()?;
        //     let used = !proof.leaf.is_zero()?;

        //     let expected = used.select(&new_root, &implied_root)?;
        //     implied_root.enforce_equal(&expected)?;
        // }

        // let input_commitment_hashes =
        //     &std::array::from_fn(|i| self.commitment_inclusion_proofs[i].leaf.clone());

        // // Verify that the operation returns the resulting outputs.
        // let operation_result = self.operation.verify(input_commitment_hashes)?;

        // Ok(AMLResultVar {
        //     nullifiers: operation_result.nullifiers,
        //     output_commitment_hashes: operation_result.output_commitment_hashes,
        //     withdrawal_amounts: std::array::from_fn(|i| {
        //         operation_result.withdrawals[i].amount.clone()
        //     }),
        //     withdrawal_assets: std::array::from_fn(|i| {
        //         operation_result.withdrawals[i].asset.clone()
        //     }),
        // })
    }
}

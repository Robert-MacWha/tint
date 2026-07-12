use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_r1cs_std::alloc::{AllocVar, AllocationMode};
use ark_relations::gr1cs::{Namespace, SynthesisError};

use crate::{
    circuit::{
        FrVar,
        merkle_tree::{InclusionProofVar, SubtreeAppendProofVar},
        operation::{OperationVar, WithdrawalVar},
        try_array_from_fn, variable,
    },
    indexer::merkle_tree::{InclusionProof, SubtreeAppendProof},
    operation::Operation,
};

const N_INPUTS: usize = 5;
const N_OUTPUTS: usize = 5;
const N_WITHDRAWALS: usize = 5;

const TREE_DEPTH: usize = 8;
const SUBTREE_DEPTH: usize = 2;
const SUBTREE_PATH_LENGTH: usize = TREE_DEPTH - SUBTREE_DEPTH;
const K: usize = 8;

const SUBTREE_SIZE: usize = K.pow(SUBTREE_DEPTH as u32);

pub struct JoinSplit {
    pub subtree_append: SubtreeAppendProof<SUBTREE_PATH_LENGTH, SUBTREE_SIZE, K>,
    pub commitment_inclusion_proofs: [InclusionProof<TREE_DEPTH, K>; N_INPUTS],
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct JoinSplitVar {
    pub subtree_append: SubtreeAppendProofVar<SUBTREE_PATH_LENGTH, SUBTREE_DEPTH, SUBTREE_SIZE, K>,
    pub commitment_inclusion_proofs: [InclusionProofVar<TREE_DEPTH, K>; N_INPUTS],
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct JoinSplitResult {
    pub new_root: FrVar,
    pub nullifiers: [FrVar; N_INPUTS],
    pub output_commitment_hashes: [FrVar; N_OUTPUTS],
    pub withdrawals: [WithdrawalVar; N_WITHDRAWALS],
}

impl JoinSplitVar {
    /// Verifies the JoinSplit operation.
    pub fn verify(
        &self,
        old_root: &FrVar,
        old_root_length: &FrVar,
        start_aggregation_hash: &FrVar,
        end_aggregation_hash: &FrVar,
    ) -> Result<JoinSplitResult, SynthesisError> {
        // Verify the staged leaf append proof and return the new root of the Merkle tree.
        let new_root = self.subtree_append.verify(
            old_root,
            old_root_length,
            start_aggregation_hash,
            end_aggregation_hash,
        )?;

        // Verify the inclusion proofs for the input commitments.
        for proof in &self.commitment_inclusion_proofs {
            proof.verify_membership(&new_root)?;
        }

        let input_commitment_hashes =
            &std::array::from_fn(|i| self.commitment_inclusion_proofs[i].leaf.clone());

        // Verify that the operation is balanced and returns the resulting outputs.
        let operation_result = self.operation.verify(input_commitment_hashes)?;

        Ok(JoinSplitResult {
            new_root,
            nullifiers: operation_result.nullifiers,
            output_commitment_hashes: operation_result.output_commitment_hashes,
            withdrawals: operation_result.withdrawals,
        })
    }
}

impl AllocVar<JoinSplit, Fr> for JoinSplitVar {
    fn new_variable<T: Borrow<JoinSplit>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let subtree_append = variable(cs.clone(), &value.subtree_append, mode)?;
        let commitment_inclusion_proofs = try_array_from_fn(|i| {
            variable(cs.clone(), &value.commitment_inclusion_proofs[i], mode)
        })?;
        let operation = variable(cs.clone(), &value.operation, mode)?;

        Ok(Self {
            subtree_append,
            commitment_inclusion_proofs,
            operation,
        })
    }
}

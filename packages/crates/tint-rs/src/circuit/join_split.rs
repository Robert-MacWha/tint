use std::borrow::Borrow;

use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use ark_r1cs_std::{
    GR1CSVar,
    alloc::{AllocVar, AllocationMode},
    eq::EqGadget,
    fields::FieldVar,
};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, Namespace, OptimizationGoal,
    SynthesisError,
};

use crate::{
    array::try_from_fn,
    circuit::{
        FrVar, input,
        merkle_tree::{InclusionProofVar, SubtreeAppendProofVar},
        operation::OperationVar,
        output, variable, witness,
    },
    indexer::{
        fr_to_address,
        merkle_tree::{InclusionProof, SubtreeAppendProof},
    },
    note::asset::AssetId,
    operation::Operation,
};

pub const N_INPUTS: usize = 5;
pub const N_OUTPUTS: usize = 5;
pub const N_WITHDRAWALS: usize = 2;

pub const TREE_DEPTH: usize = 8;
pub const SUBTREE_DEPTH: usize = 2;
pub const SUBTREE_PATH_LENGTH: usize = TREE_DEPTH - SUBTREE_DEPTH;
pub const K: usize = 8;

pub const SUBTREE_SIZE: usize = K.pow(SUBTREE_DEPTH as u32);

#[derive(Clone, Default)]
pub struct JoinSplit {
    // Public inputs
    pub old_root: Fr,
    pub start_aggregation_index: u128,
    pub start_aggregation_hash: Fr,
    pub bound_params_hash: Fr,

    // Witnessed values
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
    pub new_root: Fr,
    pub end_aggregation_hash: Fr,
    pub nullifiers: [Fr; N_INPUTS],
    pub spendability_addresses: [Address; N_INPUTS],
    pub output_commitment_hashes: [Fr; N_OUTPUTS],
    pub withdrawal_amounts: [u128; N_WITHDRAWALS],
    pub withdrawal_assets: [AssetId; N_WITHDRAWALS],
}

pub struct JoinSplitResultVar {
    pub new_root: FrVar,
    pub end_aggregation_hash: FrVar,
    pub nullifiers: [FrVar; N_INPUTS],
    pub spendability_addresses: [FrVar; N_INPUTS],
    pub output_commitment_hashes: [FrVar; N_OUTPUTS],
    pub withdrawal_amounts: [FrVar; N_WITHDRAWALS],
    pub withdrawal_assets: [FrVar; N_WITHDRAWALS],
}

impl JoinSplit {
    pub fn new(
        old_root: Fr,
        start_aggregation_index: u128,
        start_aggregation_hash: Fr,
        bound_params_hash: Fr,
        subtree_append: SubtreeAppendProof<SUBTREE_PATH_LENGTH, SUBTREE_SIZE, K>,
        commitment_inclusion_proofs: [InclusionProof<TREE_DEPTH, K>; N_INPUTS],
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    ) -> Self {
        Self {
            old_root,
            start_aggregation_index,
            start_aggregation_hash,
            bound_params_hash,
            subtree_append,
            commitment_inclusion_proofs,
            operation,
        }
    }

    /// Synthesizes the JoinSplit circuit, returning the public inputs (in
    /// the order matching `ProofLib.toPublicSignals` / `N_PUB`).
    pub fn synthesize_public_inputs(&self) -> Result<Vec<Fr>, SynthesisError> {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);

        let _ = self.synthesize(cs.clone())?;
        cs.finalize();

        // `instance_assignment()` leads with the implicit constant-1 term;
        // callers (and `Groth16::verify`) only want the actual signals.
        Ok(cs.instance_assignment()?[1..].to_vec())
    }

    /// Synthesizes the JoinSplit circuit, returning the public outputs.
    pub fn synthesize_outputs(&self) -> Result<JoinSplitResult, SynthesisError> {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);

        let result = self.synthesize(cs.clone())?;
        cs.finalize();

        result.try_into()
    }

    fn synthesize(
        &self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<JoinSplitResultVar, SynthesisError> {
        // Public inputs
        let old_root = input(cs.clone(), &self.old_root)?;
        let start_aggregation_index = input(cs.clone(), &self.start_aggregation_index.into())?;
        let start_aggregation_hash = input(cs.clone(), &self.start_aggregation_hash)?;
        let _bound_params_hash: FrVar = input(cs.clone(), &self.bound_params_hash)?;

        // Witnessed values
        let join_split_var: JoinSplitVar = witness(cs.clone(), self)?;

        let result =
            join_split_var.verify(&old_root, &start_aggregation_index, &start_aggregation_hash)?;

        // Public outputs
        // TODO: Find a way to enforce all outputs are properly constrained, rather
        // than doing this manually.  Maybe have `output` be the fn that converts
        // from `FrVar` to `Fr`?
        output(cs.clone(), &result.new_root)?;
        output(cs.clone(), &result.end_aggregation_hash)?;
        for i in 0..N_INPUTS {
            output(cs.clone(), &result.nullifiers[i])?;
            output(cs.clone(), &result.spendability_addresses[i])?;
        }
        for i in 0..N_OUTPUTS {
            output(cs.clone(), &result.output_commitment_hashes[i])?;
        }
        for i in 0..N_WITHDRAWALS {
            output(cs.clone(), &result.withdrawal_amounts[i])?;
            output(cs.clone(), &result.withdrawal_assets[i])?;
        }

        Ok(result)
    }
}

impl ConstraintSynthesizer<Fr> for JoinSplit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let _ = self.synthesize(cs)?;
        Ok(())
    }
}

impl JoinSplitVar {
    /// Verifies the JoinSplit operation.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify(
        &self,
        old_root: &FrVar,
        start_aggregation_index: &FrVar,
        start_aggregation_hash: &FrVar,
    ) -> Result<JoinSplitResultVar, SynthesisError> {
        // Verify the staged leaf append proof and return the new root of the Merkle tree.
        let subtree_append_result = self.subtree_append.verify(
            old_root,
            start_aggregation_index,
            start_aggregation_hash,
        )?;
        let new_root = subtree_append_result.new_root;

        // Verify the inclusion proofs for the input commitments. Skipped for
        // zero-valued leaves (padding).
        for proof in &self.commitment_inclusion_proofs {
            let implied_root = proof.root()?;
            let used = !proof.leaf.is_zero()?;

            let expected = used.select(&new_root, &implied_root)?;
            implied_root.enforce_equal(&expected)?;
        }

        let input_commitment_hashes =
            &std::array::from_fn(|i| self.commitment_inclusion_proofs[i].leaf.clone());

        // Verify that the operation is balanced and returns the resulting outputs.
        let operation_result = self.operation.verify(input_commitment_hashes)?;

        Ok(JoinSplitResultVar {
            new_root,
            end_aggregation_hash: subtree_append_result.end_aggregation_hash,
            nullifiers: operation_result.nullifiers,
            spendability_addresses: operation_result.spendability_addresses,
            output_commitment_hashes: operation_result.output_commitment_hashes,
            withdrawal_amounts: std::array::from_fn(|i| {
                operation_result.withdrawals[i].amount.clone()
            }),
            withdrawal_assets: std::array::from_fn(|i| {
                operation_result.withdrawals[i].asset.clone()
            }),
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
        let commitment_inclusion_proofs =
            try_from_fn(|i| variable(cs.clone(), &value.commitment_inclusion_proofs[i], mode))?;
        let operation = variable(cs.clone(), &value.operation, mode)?;

        Ok(Self {
            subtree_append,
            commitment_inclusion_proofs,
            operation,
        })
    }
}

impl TryFrom<JoinSplitResultVar> for JoinSplitResult {
    type Error = SynthesisError;

    fn try_from(value: JoinSplitResultVar) -> Result<Self, Self::Error> {
        let new_root = value.new_root.value()?;
        let end_aggregation_hash = value.end_aggregation_hash.value()?;
        let nullifiers = try_from_fn(|i| value.nullifiers[i].value())?;
        let spendability_addresses: [Fr; N_INPUTS] =
            try_from_fn(|i| value.spendability_addresses[i].value())?;
        let output_commitment_hashes = try_from_fn(|i| value.output_commitment_hashes[i].value())?;
        let withdrawal_amounts: [Fr; N_WITHDRAWALS] =
            try_from_fn(|i| value.withdrawal_amounts[i].value())?;
        let withdrawal_assets: [Fr; N_WITHDRAWALS] =
            try_from_fn(|i| value.withdrawal_assets[i].value())?;

        let spendability_addresses =
            std::array::from_fn(|i| fr_to_address(spendability_addresses[i]));
        let withdrawal_amounts = std::array::from_fn(|i| fr_to_u128(&withdrawal_amounts[i]));
        let withdrawal_assets = std::array::from_fn(|i| withdrawal_assets[i].into());

        Ok(Self {
            new_root,
            end_aggregation_hash,
            nullifiers,
            spendability_addresses,
            output_commitment_hashes,
            withdrawal_amounts,
            withdrawal_assets,
        })
    }
}

fn fr_to_u128(fr: &Fr) -> u128 {
    let bytes = fr.into_bigint().to_bytes_le();
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes[..16]);
    u128::from_le_bytes(arr)
}

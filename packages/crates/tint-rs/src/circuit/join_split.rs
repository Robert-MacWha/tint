use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_r1cs_std::alloc::{AllocVar, AllocationMode};
use ark_relations::gr1cs::{Namespace, SynthesisError};

use crate::{
    circuit::{
        FrVar,
        merkle_tree_subtree_append::SubtreeAppendProofVar,
        operation::OperationVar,
        poseidon::{PoseidonHasher, PoseidonHasherGadget},
    },
    indexer::merkle_tree::SubtreeAppendProof,
    operation::Operation,
};

pub struct JoinSplit<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize> {
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub subtree_append: SubtreeAppendProof<8, 2, 64>,
    pub merkle_tree_hasher: PoseidonHasher<8>,
}

pub struct JoinSplitVar<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize> {
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub subtree_append: SubtreeAppendProofVar<6, 8, 2, 64>,
    pub merkle_tree_hasher: PoseidonHasherGadget<8>,
}

impl<const I: usize, const O: usize, const W: usize> JoinSplitVar<I, O, W> {
    pub fn verify(
        &self,
        old_root: &FrVar,
        old_root_length: &FrVar,
        start_aggregation_hash: &FrVar,
        end_aggregation_hash: &FrVar,
        nullifiers: [FrVar; I],
        out_commitments: [FrVar; O],
        unshield_amounts: [FrVar; W],
        unshield_assets: [FrVar; W],
    ) -> Result<FrVar, SynthesisError> {
        todo!()
    }
}

impl<const I: usize, const O: usize, const W: usize> AllocVar<JoinSplit<I, O, W>, Fr>
    for JoinSplitVar<I, O, W>
{
    fn new_variable<T: Borrow<JoinSplit<I, O, W>>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();
        todo!()
    }
}

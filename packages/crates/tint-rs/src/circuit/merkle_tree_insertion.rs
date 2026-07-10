use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    eq::EqGadget,
};
use ark_relations::gr1cs::{Namespace, SynthesisError};

use crate::{
    circuit::{FrVar, merkle_tree_inclusion::InclusionProofVar, poseidon::PoseidonHasherGadget},
    indexer::merkle_tree::InclusionProof,
};

/// Insertion proof for a node in a Merkle tree.
pub struct InsertionProof<const D: usize, const K: usize> {
    pub old: InclusionProof<K, D>,
    pub new: InclusionProof<K, D>,
}

pub struct InsertionProofVar<const D: usize, const K: usize> {
    pub old: InclusionProofVar<D, K>,
    pub new: InclusionProofVar<D, K>,
}

impl<const D: usize, const K: usize> InsertionProofVar<D, K> {
    /// Verifies the insertion proof in-circuit, returning the new root after insertion.
    pub fn insert(
        &self,
        old_root: &FrVar,
        hasher: &PoseidonHasherGadget<K>,
    ) -> Result<FrVar, SynthesisError> {
        self.old.verify_membership(old_root, hasher)?;
        self.new.root(hasher)
    }
}

impl<const D: usize, const K: usize> AllocVar<InsertionProof<D, K>, Fr>
    for InsertionProofVar<D, K>
{
    fn new_variable<T: Borrow<InsertionProof<D, K>>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let old = InclusionProofVar::<D, K>::new_variable(cs.clone(), || Ok(&value.old), mode)?;
        let new = InclusionProofVar::<D, K>::new_variable(cs.clone(), || Ok(&value.new), mode)?;

        Ok(Self { old, new })
    }
}

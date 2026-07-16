use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    eq::EqGadget,
    prelude::Boolean,
    uint8::UInt8,
};
use ark_relations::gr1cs::{Namespace, SynthesisError};

use crate::{
    array::try_from_fn,
    circuit::{FrVar, poseidon2::poseidon2_compress_gadget, variable},
    indexer::merkle_tree::InclusionProof,
};

/// Inclusion proof for a Merkle tree of depth `D` and arity `K`. Each `path`
/// entry is a range-checked base-`K` digit (`K` must be a power of two).
pub struct InclusionProofVar<const D: usize, const K: usize> {
    pub leaf: FrVar,
    pub path: [UInt8<Fr>; D],
    pub siblings: [[FrVar; K]; D],
}

impl<const D: usize, const K: usize> InclusionProofVar<D, K> {
    pub fn new(leaf: FrVar, path: [UInt8<Fr>; D], siblings: [[FrVar; K]; D]) -> Self {
        Self {
            leaf,
            path,
            siblings,
        }
    }

    /// Verifies the inclusion proof in circuit.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify_membership(&self, root: &FrVar) -> Result<(), SynthesisError> {
        let computed_root = self.root()?;
        computed_root.enforce_equal(root)
    }

    /// Compute the root implied by this inclusion proof.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn root(&self) -> Result<FrVar, SynthesisError> {
        let mut current_hash = self.leaf.clone();

        for (digit, sibling_hashes) in self.path.iter().rev().zip(self.siblings.iter().rev()) {
            let selector = Self::one_hot_selector(digit)?;
            let input: [FrVar; K] =
                try_from_fn(|i| selector[i].select(&current_hash, &sibling_hashes[i]))?;
            current_hash = poseidon2_compress_gadget(&input)?;
        }

        Ok(current_hash)
    }

    /// Decodes a single digit into a one-hot selector of length `K`.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn one_hot_selector(digit: &UInt8<Fr>) -> Result<[Boolean<Fr>; K], SynthesisError> {
        const { assert!(K.is_power_of_two(), "arity must be a power of two") };

        let used_bits = K.trailing_zeros() as usize;
        for bit in &digit.bits[used_bits..] {
            bit.enforce_equal(&Boolean::FALSE)?;
        }

        try_from_fn(|k| digit.is_eq(&UInt8::constant(k as u8)))
    }
}

impl<const D: usize, const K: usize> AllocVar<InclusionProof<D, K>, Fr>
    for InclusionProofVar<D, K>
{
    fn new_variable<T: Borrow<InclusionProof<D, K>>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let leaf = variable(cs.clone(), &value.leaf, mode)?;
        let path = try_from_fn(|i| variable(cs.clone(), &value.path[i], mode))?;
        let siblings =
            try_from_fn(|i| try_from_fn(|j| variable(cs.clone(), &value.siblings[i][j], mode)))?;

        Ok(Self {
            leaf,
            siblings,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;

    use super::*;
    use crate::{circuit::witness, indexer::merkle_tree::IncrementalMerkleTree};

    /// Expect that the inclusion proof verifies correctly in circuit.
    #[test]
    fn verify_membership() {
        let native_leaves = (0..32).map(Fr::from).collect::<Vec<_>>();
        let tree = IncrementalMerkleTree::<5, 2>::from_leaves(&native_leaves);
        let root = tree.root();

        let target = native_leaves[7];
        let target_path = tree.path(target).unwrap();
        let inclusion_proof = tree.inclusion(target_path);

        let cs = ConstraintSystem::<Fr>::new_ref();
        let proof_var: InclusionProofVar<5, 2> = witness(cs.clone(), &inclusion_proof).unwrap();

        let root_var = witness(cs.clone(), &root).unwrap();
        proof_var.verify_membership(&root_var).unwrap();

        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that an invalid inclusion proof fails to verify in circuit.
    #[test]
    fn invalid_proof() {
        let native_leaves = (0..32).map(Fr::from).collect::<Vec<_>>();
        let tree = IncrementalMerkleTree::<5, 2>::from_leaves(&native_leaves);
        let root = tree.root();

        let target = native_leaves[7];
        let target_path = tree.path(target).unwrap();
        let mut inclusion_proof = tree.inclusion(target_path);
        inclusion_proof.path[0] = 1u8; // corrupt the proof

        let cs = ConstraintSystem::<Fr>::new_ref();
        let proof_var: InclusionProofVar<5, 2> = witness(cs.clone(), &inclusion_proof).unwrap();

        let root_var = witness(cs.clone(), &root).unwrap();
        proof_var.verify_membership(&root_var).unwrap();

        assert!(!cs.is_satisfied().unwrap());
    }

    /// Expect that the one-hot selector is computed correctly for a valid digit.
    #[test]
    fn onehot_selector() {
        const K: usize = 4;

        let cs = ConstraintSystem::<Fr>::new_ref();
        let digit: UInt8<Fr> = witness(cs.clone(), &2u8).unwrap();
        let selector = InclusionProofVar::<2, K>::one_hot_selector(&digit).unwrap();

        assert!(selector[0].value().unwrap() == false);
        assert!(selector[1].value().unwrap() == false);
        assert!(selector[2].value().unwrap() == true);
        assert!(selector[3].value().unwrap() == false);

        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that the one-hot selector fails for a digit that is out of range.
    #[test]
    fn onehot_out_of_range() {
        const K: usize = 4;

        let cs = ConstraintSystem::<Fr>::new_ref();
        let digit: UInt8<Fr> = witness(cs.clone(), &((K + 1) as u8)).unwrap();
        let _ = InclusionProofVar::<2, K>::one_hot_selector(&digit).unwrap();

        assert!(!cs.is_satisfied().unwrap());
    }
}

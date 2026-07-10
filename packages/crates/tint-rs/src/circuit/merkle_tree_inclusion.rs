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
    circuit::{FrVar, poseidon::PoseidonHasherGadget, try_array_from_fn, variable},
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
    pub fn verify_membership(
        &self,
        root: &FrVar,
        hasher: &PoseidonHasherGadget<K>,
    ) -> Result<(), SynthesisError> {
        let computed_root = self.root(hasher)?;
        computed_root.enforce_equal(root)
    }

    /// Computes the root of the merkle tree asuming the leaf and path on `self` are correct.
    pub fn root(&self, hasher: &PoseidonHasherGadget<K>) -> Result<FrVar, SynthesisError> {
        let mut current_hash = self.leaf.clone();

        for (digit, sibling_hashes) in self.path.iter().rev().zip(self.siblings.iter().rev()) {
            let selector = Self::one_hot_selector(digit)?;
            let input =
                try_array_from_fn(|i| selector[i].select(&current_hash, &sibling_hashes[i]))?;
            current_hash = hasher.hash(&input)?;
        }

        Ok(current_hash)
    }

    /// Decodes a single digit into a one-hot selector of length `K`.
    fn one_hot_selector(digit: &UInt8<Fr>) -> Result<[Boolean<Fr>; K], SynthesisError> {
        const { assert!(K.is_power_of_two(), "arity must be a power of two") };

        let used_bits = K.trailing_zeros() as usize;
        for bit in &digit.bits[used_bits..] {
            bit.enforce_equal(&Boolean::FALSE)?;
        }

        try_array_from_fn(|k| digit.is_eq(&UInt8::constant(k as u8)))
    }
}

impl<const D: usize, const K: usize> AllocVar<InclusionProof<K, D>, Fr>
    for InclusionProofVar<D, K>
{
    fn new_variable<T: Borrow<InclusionProof<K, D>>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let leaf = variable(cs.clone(), value.leaf, mode)?;
        let path =
            try_array_from_fn(|i| UInt8::new_variable(cs.clone(), || Ok(value.path[i]), mode))?;
        let siblings = try_array_from_fn(|i| {
            try_array_from_fn(|j| variable(cs.clone(), value.siblings[i][j].clone(), mode))
        })?;

        Ok(Self {
            leaf,
            siblings,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use ark_ff::Zero;
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;

    use super::*;
    use crate::circuit::poseidon::PoseidonHasher;

    #[test]
    fn allocates_and_verifies_when_depth_and_arity_differ() {
        const K: usize = 2;
        const D: usize = 5;

        let native_hasher = PoseidonHasher::<K>::new().unwrap();
        let siblings = [[Fr::from(1u64), Fr::from(2u64)]; D];
        let leaf = Fr::from(7u64);
        let path = [0u8; D];

        let mut root = leaf;
        for group in siblings.iter().rev() {
            let mut input = *group;
            input[0] = root;
            root = native_hasher.hash(input);
        }

        let proof = InclusionProof::<K, D> {
            path,
            siblings,
            leaf,
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        let hasher = PoseidonHasherGadget::<K>::new_variable(
            cs.clone(),
            || Ok(&native_hasher),
            AllocationMode::Constant,
        )
        .unwrap();
        let proof_var = InclusionProofVar::<D, K>::new_variable(
            cs.clone(),
            || Ok(&proof),
            AllocationMode::Witness,
        )
        .unwrap();
        let root_var = variable(cs.clone(), root, AllocationMode::Input).unwrap();

        proof_var.verify_membership(&root_var, &hasher).unwrap();

        assert!(cs.is_satisfied().unwrap());
        assert_ne!(D, K, "regression test must exercise D != K");
        assert_ne!(root_var.value().unwrap(), Fr::zero());
    }

    #[test]
    fn rejects_out_of_range_digit() {
        const K: usize = 2;
        const D: usize = 3;

        let siblings = [[Fr::from(1u64), Fr::from(2u64)]; D];
        let leaf = Fr::from(7u64);
        // 2 is out of range for arity K=2 (valid digits are 0 or 1); a prover
        // could otherwise use this to drop `current_hash` from the hash input
        // entirely, as neither slot 0 nor slot 1 would be selected.
        let proof = InclusionProof::<K, D> {
            path: [2u8; D],
            siblings,
            leaf,
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        let proof_var = InclusionProofVar::<D, K>::new_variable(
            cs.clone(),
            || Ok(&proof),
            AllocationMode::Witness,
        )
        .unwrap();
        let native_hasher = PoseidonHasher::<K>::new().unwrap();
        let hasher = PoseidonHasherGadget::<K>::new_variable(
            cs.clone(),
            || Ok(&native_hasher),
            AllocationMode::Constant,
        )
        .unwrap();

        let _ = proof_var.root(&hasher).unwrap();

        assert!(!cs.is_satisfied().unwrap());
    }
}

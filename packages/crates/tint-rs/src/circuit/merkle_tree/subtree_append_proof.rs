use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    cmp::CmpGadget,
    eq::EqGadget,
    fields::FieldVar,
    prelude::Boolean,
    uint8::UInt8,
};
use ark_relations::gr1cs::{Namespace, SynthesisError};

use crate::{
    circuit::{
        FrVar, merkle_tree::InclusionProofVar, merkle_tree::root_proof,
        poseidon::poseidon_hash_gadget, try_array_from_fn, variable,
    },
    indexer::merkle_tree::SubtreeAppendProof,
};

/// Proof for appending up to `SUBTREE_SIZE` leaves into a Merkle tree of
/// depth `D` and arity `K`.
pub struct SubtreeAppendProofVar<
    // Number of levels from the root to the subtree being appended.
    const SUBTREE_PATH_LEN: usize,
    // Depth of the subtree being appended.
    const SUBTREE_DEPTH: usize,
    // Number of leaves in the subtree being appended.
    const SUBTREE_SIZE: usize,
    // Arity of the Merkle tree.
    const K: usize,
> {
    pub existing_leaves: [FrVar; SUBTREE_SIZE],
    pub new_leaves: [FrVar; SUBTREE_SIZE],
    pub current_siblings: [[FrVar; K]; SUBTREE_PATH_LEN],
    pub next_siblings: [[FrVar; K]; SUBTREE_PATH_LEN],
}

pub struct SubtreeAppendProofResult {
    pub new_root: FrVar,
    pub end_aggregation_hash: FrVar,
}

impl<
    const SUBTREE_PATH_LEN: usize,
    const SUBTREE_DEPTH: usize,
    const SUBTREE_SIZE: usize,
    const K: usize,
> SubtreeAppendProofVar<SUBTREE_PATH_LEN, SUBTREE_DEPTH, SUBTREE_SIZE, K>
{
    /// Verifies that the new leaves can be appended into the Merkle tree after
    /// `old_root_length` leaves, and returns the new root and the new aggregation hash.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify(
        &self,
        old_root: &FrVar,
        old_root_length: &FrVar,
        start_aggregation_hash: &FrVar,
    ) -> Result<SubtreeAppendProofResult, SynthesisError> {
        let end_aggregation_hash = self.verify_aggregation_hash(start_aggregation_hash)?;
        let new_root = self.verify_inclusion(old_root, old_root_length)?;
        Ok(SubtreeAppendProofResult {
            new_root,
            end_aggregation_hash,
        })
    }

    /// Verifies that the new leaves can be appended into the Merkle tree after
    /// `old_root_length` leaves, and returns the new aggregation hash.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn verify_aggregation_hash(
        &self,
        start_aggregation_hash: &FrVar,
    ) -> Result<FrVar, SynthesisError> {
        let mut aggregation_hash: FrVar = start_aggregation_hash.clone();
        for new_leaf in &self.new_leaves {
            let next_aggregation_hash: FrVar =
                poseidon_hash_gadget(&[aggregation_hash.clone(), new_leaf.clone()])?;
            aggregation_hash = new_leaf
                .is_zero()?
                .select(&aggregation_hash, &next_aggregation_hash)?;
        }

        Ok(aggregation_hash)
    }

    /// Verifies the append proof that `new_leaves` can be inserted into the tree
    /// after `old_root_length` leaves.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn verify_inclusion(
        &self,
        old_root: &FrVar,
        old_root_length: &FrVar,
    ) -> Result<FrVar, SynthesisError> {
        let (filled, current_path) = Self::locate(old_root_length)?;
        let (_, next_path) =
            Self::locate(&(old_root_length + FrVar::constant(Fr::from(SUBTREE_SIZE as u64))))?;

        let new_current_leaves = self.merge_current(&filled)?;
        let new_next_leaves = self.merge_next(&filled)?;

        //? Verify the insertion for the current subtree
        let current_root_before =
            root_proof::<SUBTREE_DEPTH, K, SUBTREE_SIZE>(&self.existing_leaves)?;
        let current_root_after = root_proof::<SUBTREE_DEPTH, K, SUBTREE_SIZE>(&new_current_leaves)?;
        let intermediate_root = Self::update_subtree(
            old_root,
            &current_path,
            &self.current_siblings,
            current_root_before,
            current_root_after,
        )?;

        //? Verify the insertion for the next subtree, if any
        let next_root_before = Self::empty_root()?;
        let next_root_after = root_proof::<SUBTREE_DEPTH, K, SUBTREE_SIZE>(&new_next_leaves)?;
        Self::update_subtree(
            &intermediate_root,
            &next_path,
            &self.next_siblings,
            next_root_before,
            next_root_after,
        )
    }

    /// Checks that the subtree at `path` currently has a value of `before` and
    /// returns the new root after updating it to `after`.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn update_subtree(
        root: &FrVar,
        path: &[UInt8<Fr>; SUBTREE_PATH_LEN],
        siblings: &[[FrVar; K]; SUBTREE_PATH_LEN],
        before: FrVar,
        after: FrVar,
    ) -> Result<FrVar, SynthesisError> {
        InclusionProofVar::new(before, path.clone(), siblings.clone()).verify_membership(root)?;
        InclusionProofVar::new(after, path.clone(), siblings.clone()).root()
    }

    /// Splits a leaf count into the fill level of its subtree (the low
    /// `SUBTREE_DEPTH` base-`K` digits) and that subtree's path to the root
    /// (the remaining `SUBTREE_PATH_LEN` digits), matching
    /// `IncrementalMerkleTree::path_for_index`'s digit order.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn locate(
        length: &FrVar,
    ) -> Result<(UInt8<Fr>, [UInt8<Fr>; SUBTREE_PATH_LEN]), SynthesisError> {
        const {
            assert!(K.is_power_of_two(), "arity must be a power of two");
            assert!(
                K.trailing_zeros() as usize * SUBTREE_DEPTH <= 8,
                "SUBTREE_SIZE must fit in a u8 for `filled` to be representable as a UInt8"
            );
        }
        let bits_per_digit = K.trailing_zeros() as usize;
        let low_bits = bits_per_digit * SUBTREE_DEPTH;
        let total_bits = bits_per_digit * (SUBTREE_DEPTH + SUBTREE_PATH_LEN);

        let (bits, _) = length.to_bits_le_with_top_bits_zero(total_bits)?;
        let mut filled_bits = [Boolean::FALSE; 8];
        filled_bits[..low_bits].clone_from_slice(&bits[..low_bits]);
        let filled = UInt8::from_bits_le(&filled_bits);

        let path = try_array_from_fn(|i| {
            let m = SUBTREE_PATH_LEN - 1 - i;
            let digit_bits =
                &bits[low_bits + m * bits_per_digit..low_bits + (m + 1) * bits_per_digit];
            let mut padded = [Boolean::FALSE; 8];
            padded[..bits_per_digit].clone_from_slice(digit_bits);
            Ok(UInt8::from_bits_le(&padded))
        })?;

        Ok((filled, path))
    }

    /// Merges the existing leaves with the new leaves, masking in only the
    /// new leaves that fit into the current subtree.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn merge_current(&self, filled: &UInt8<Fr>) -> Result<[FrVar; SUBTREE_SIZE], SynthesisError> {
        let fill_eq = Self::fill_indicators(filled)?;

        try_array_from_fn(|pos| {
            let is_existing = UInt8::constant(pos as u8).is_lt(filled)?;
            let new_leaf = self.shifted_new_leaf(&fill_eq, pos)?;
            is_existing.select(&self.existing_leaves[pos], &new_leaf)
        })
    }

    /// Merges the new leaves into the next subtree, masking in only the new
    /// leaves that overflowed from the current subtree.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn merge_next(&self, filled: &UInt8<Fr>) -> Result<[FrVar; SUBTREE_SIZE], SynthesisError> {
        let fill_eq = Self::fill_indicators(filled)?;
        try_array_from_fn(|pos| self.shifted_new_leaf(&fill_eq, SUBTREE_SIZE + pos))
    }

    /// `new_leaves[target - d]` masked in for whichever `d` matches `filled`
    /// (contributes zero if `target - d` is out of range).
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn shifted_new_leaf(
        &self,
        fill_eq: &[Boolean<Fr>; SUBTREE_SIZE],
        target: usize,
    ) -> Result<FrVar, SynthesisError> {
        let mut sum = FrVar::zero();
        for (d, eq) in fill_eq.iter().enumerate() {
            if let Some(j) = target.checked_sub(d).filter(|&j| j < SUBTREE_SIZE) {
                sum += eq.select(&self.new_leaves[j], &FrVar::zero())?;
            }
        }
        Ok(sum)
    }

    /// One-hot indicator over the only values `filled` can take:
    /// `fill_eq[d]` is true if `filled == d`.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn fill_indicators(filled: &UInt8<Fr>) -> Result<[Boolean<Fr>; SUBTREE_SIZE], SynthesisError> {
        try_array_from_fn(|d| filled.is_eq(&UInt8::constant(d as u8)))
    }

    /// Computes the root of an empty tree.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn empty_root() -> Result<FrVar, SynthesisError> {
        let mut root = FrVar::zero();
        for _ in 0..SUBTREE_DEPTH {
            let siblings: [FrVar; K] = std::array::repeat(root.clone());
            root = poseidon_hash_gadget(&siblings)?;
        }
        Ok(root)
    }
}

impl<
    const SUBTREE_PATH_LEN: usize,
    const SUBTREE_DEPTH: usize,
    const SUBTREE_SIZE: usize,
    const K: usize,
> AllocVar<SubtreeAppendProof<SUBTREE_PATH_LEN, SUBTREE_SIZE, K>, Fr>
    for SubtreeAppendProofVar<SUBTREE_PATH_LEN, SUBTREE_DEPTH, SUBTREE_SIZE, K>
{
    fn new_variable<T: Borrow<SubtreeAppendProof<SUBTREE_PATH_LEN, SUBTREE_SIZE, K>>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        const {
            assert!(
                SUBTREE_SIZE == K.pow(SUBTREE_DEPTH as u32),
                "SUBTREE_SIZE must equal K^SUBTREE_DEPTH"
            )
        };

        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let existing_leaves =
            try_array_from_fn(|i| variable(cs.clone(), &value.existing_leaves[i], mode))?;
        let new_leaves = try_array_from_fn(|i| variable(cs.clone(), &value.new_leaves[i], mode))?;
        let current_siblings = try_array_from_fn(|i| {
            try_array_from_fn(|j| variable(cs.clone(), &value.current_siblings[i][j], mode))
        })?;
        let next_siblings = try_array_from_fn(|i| {
            try_array_from_fn(|j| variable(cs.clone(), &value.next_siblings[i][j], mode))
        })?;

        Ok(Self {
            existing_leaves,
            new_leaves,
            current_siblings,
            next_siblings,
        })
    }
}

#[cfg(test)]
mod tests {
    use ark_r1cs_std::GR1CSVar;
    use std::array::repeat;

    use super::*;

    /// Expect that merging existing and new leaves will tightly pack the new
    /// leaves into the currents subtree and overflow into the next subtree.
    #[test]
    fn test_merge() {
        let proof = SubtreeAppendProofVar::<2, 2, 4, 2> {
            existing_leaves: [
                FrVar::constant(Fr::from(1u64)),
                FrVar::constant(Fr::from(2u64)),
                FrVar::constant(Fr::from(0u64)),
                FrVar::constant(Fr::from(0u64)),
            ],
            new_leaves: [
                FrVar::constant(Fr::from(5u64)),
                FrVar::constant(Fr::from(6u64)),
                FrVar::constant(Fr::from(7u64)),
                FrVar::constant(Fr::from(8u64)),
            ],
            current_siblings: repeat(repeat(FrVar::zero())),
            next_siblings: repeat(repeat(FrVar::zero())),
        };

        let filled = UInt8::constant(2u8);
        let merged_current = proof
            .merge_current(&filled)
            .unwrap()
            .iter()
            .map(|x| x.value().unwrap())
            .collect::<Vec<_>>();

        let merged_next = proof
            .merge_next(&filled)
            .unwrap()
            .iter()
            .map(|x| x.value().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            merged_current,
            vec![
                Fr::from(1u64),
                Fr::from(2u64),
                Fr::from(5u64),
                Fr::from(6u64)
            ]
        );
        assert_eq!(
            merged_next,
            vec![
                Fr::from(7u64),
                Fr::from(8u64),
                Fr::from(0u64),
                Fr::from(0u64)
            ]
        );
    }
}

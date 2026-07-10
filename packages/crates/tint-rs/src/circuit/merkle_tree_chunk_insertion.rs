use std::{borrow::Borrow, cmp::Ordering};

use ark_bn254::Fr;
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    eq::EqGadget,
    fields::FieldVar,
    prelude::Boolean,
    uint8::UInt8,
};
use ark_relations::gr1cs::{Namespace, SynthesisError};

use crate::{
    circuit::{
        FrVar, merkle_tree_inclusion::InclusionProofVar, merkle_tree_root::merkle_root,
        poseidon::PoseidonHasherGadget, try_array_from_fn, variable,
    },
    indexer::merkle_tree::ChunkInsertionProof,
};

/// Witness for proving a batch of up to `CHUNK_SIZE` leaves was inserted into
/// a Merkle tree of arity `K`. `CHUNK_DEPTH` is the depth of a single chunk
/// (`CHUNK_SIZE == K^CHUNK_DEPTH`) and `CHUNK_PATH_LEN` is the path length
/// *above* a chunk, up to the tree root.
pub struct ChunkInsertionProofVar<
    const CHUNK_DEPTH: usize,
    const CHUNK_PATH_LEN: usize,
    const K: usize,
    const CHUNK_SIZE: usize,
> {
    pub existing_leaves: [FrVar; CHUNK_SIZE],
    pub new_leaves: [FrVar; CHUNK_SIZE],
    pub current_siblings: [[FrVar; K]; CHUNK_PATH_LEN],
    pub next_siblings: [[FrVar; K]; CHUNK_PATH_LEN],
}

impl<const CHUNK_DEPTH: usize, const CHUNK_PATH_LEN: usize, const K: usize, const CHUNK_SIZE: usize>
    ChunkInsertionProofVar<CHUNK_DEPTH, CHUNK_PATH_LEN, K, CHUNK_SIZE>
{
    /// Verifies that inserting this batch's `new_leaves` into the tree
    /// (starting right after leaf `old_root_length - 1`) transforms
    /// `old_root` into the returned new root.
    ///
    /// `old_root_length` is the number of leaves in the tree before this
    /// insertion (a public input, tracked by the caller e.g. on-chain); the
    /// chunk index and fill level are derived from it here rather than
    /// witnessed directly, which both removes a witness and bounds the fill
    /// level to `[0, CHUNK_SIZE)` for free (it is exactly the low
    /// `CHUNK_DEPTH` digits of `old_root_length`).
    pub fn insert(
        &self,
        old_root: &FrVar,
        old_root_length: &FrVar,
        hasher: &PoseidonHasherGadget<K>,
    ) -> Result<FrVar, SynthesisError> {
        let chunk_filled = Self::chunk_filled(old_root_length)?;
        let current_path = Self::path_digits(old_root_length)?;
        let next_path =
            Self::path_digits(&(old_root_length + FrVar::constant(Fr::from(CHUNK_SIZE as u64))))?;

        let merged_current = self.merge_current_chunk(&chunk_filled)?;
        let computed_next = self.merge_next_chunk(&chunk_filled)?;

        let old_current_root =
            merkle_root::<CHUNK_DEPTH, K, CHUNK_SIZE>(&self.existing_leaves, hasher)?;
        let new_current_root = merkle_root::<CHUNK_DEPTH, K, CHUNK_SIZE>(&merged_current, hasher)?;
        let new_next_root = merkle_root::<CHUNK_DEPTH, K, CHUNK_SIZE>(&computed_next, hasher)?;
        let zero_root = Self::empty_chunk_root(hasher)?;

        InclusionProofVar::new(
            old_current_root,
            current_path.clone(),
            self.current_siblings.clone(),
        )
        .verify_membership(old_root, hasher)?;
        let intermediate_root = InclusionProofVar::new(
            new_current_root,
            current_path,
            self.current_siblings.clone(),
        )
        .root(hasher)?;

        InclusionProofVar::new(zero_root, next_path.clone(), self.next_siblings.clone())
            .verify_membership(&intermediate_root, hasher)?;
        InclusionProofVar::new(new_next_root, next_path, self.next_siblings.clone()).root(hasher)
    }

    /// Recombines the low `CHUNK_DEPTH` base-`K` digits of `old_root_length`
    /// into a value in `[0, CHUNK_SIZE)` — the number of leaves already
    /// present in the current chunk.
    fn chunk_filled(old_root_length: &FrVar) -> Result<FrVar, SynthesisError> {
        let (bits, _) = old_root_length.to_bits_le_with_top_bits_zero(Self::total_bits())?;
        Ok(Boolean::le_bits_to_fp(&bits[..Self::low_bits()])?)
    }

    /// Decomposes the high `CHUNK_PATH_LEN` base-`K` digits of `length`
    /// (`length`'s chunk index) into a path, root-first, matching
    /// `IncrementalMerkleTree::path_for_index`'s digit ordering.
    fn path_digits(length: &FrVar) -> Result<[UInt8<Fr>; CHUNK_PATH_LEN], SynthesisError> {
        let (bits, _) = length.to_bits_le_with_top_bits_zero(Self::total_bits())?;
        let bits_per_level = Self::bits_per_level();
        let low_bits = Self::low_bits();

        try_array_from_fn(|i| {
            let m = CHUNK_PATH_LEN - 1 - i;
            let group = &bits[low_bits + m * bits_per_level..low_bits + (m + 1) * bits_per_level];
            let mut padded = [Boolean::FALSE; 8];
            padded[..bits_per_level].clone_from_slice(group);
            Ok(UInt8::from_bits_le(&padded))
        })
    }

    /// Merges `existing_leaves` and the portion of `new_leaves` that lands in
    /// the current chunk: position `pos` keeps `existing_leaves[pos]` while
    /// `pos < chunk_filled`, and otherwise takes `new_leaves[pos -
    /// chunk_filled]` (zero if `new_leaves` doesn't reach that far).
    fn merge_current_chunk(
        &self,
        chunk_filled: &FrVar,
    ) -> Result<[FrVar; CHUNK_SIZE], SynthesisError> {
        try_array_from_fn(|pos| {
            let pos_fr = FrVar::constant(Fr::from(pos as u64));
            let is_existing = pos_fr.is_cmp_unchecked(chunk_filled, Ordering::Less, false)?;

            let mut new_contribution = FrVar::zero();
            for (j, new_leaf) in self.new_leaves.iter().enumerate() {
                let target = chunk_filled + FrVar::constant(Fr::from(j as u64));
                let is_target = target.is_eq(&pos_fr)?;
                new_contribution += is_target.select(new_leaf, &FrVar::zero())?;
            }

            is_existing.select(&self.existing_leaves[pos], &new_contribution)
        })
    }

    /// Computes the portion of `new_leaves` that overflows into the next
    /// chunk: position `pos` takes `new_leaves[CHUNK_SIZE - chunk_filled +
    /// pos]` if that index is within bounds, else zero.
    fn merge_next_chunk(
        &self,
        chunk_filled: &FrVar,
    ) -> Result<[FrVar; CHUNK_SIZE], SynthesisError> {
        try_array_from_fn(|pos| {
            let target_value = FrVar::constant(Fr::from((CHUNK_SIZE + pos) as u64));

            let mut contribution = FrVar::zero();
            for (j, new_leaf) in self.new_leaves.iter().enumerate() {
                let target = chunk_filled + FrVar::constant(Fr::from(j as u64));
                let is_target = target.is_eq(&target_value)?;
                contribution += is_target.select(new_leaf, &FrVar::zero())?;
            }

            Ok(contribution)
        })
    }

    /// The root of an entirely-empty `CHUNK_DEPTH`-deep subtree, matching
    /// `IncrementalMerkleTree::compute_zeros`'s native equivalent.
    fn empty_chunk_root(hasher: &PoseidonHasherGadget<K>) -> Result<FrVar, SynthesisError> {
        let mut root = FrVar::zero();
        for _ in 0..CHUNK_DEPTH {
            root = hasher.hash(&std::array::from_fn(|_| root.clone()))?;
        }
        Ok(root)
    }

    fn low_bits() -> usize {
        Self::bits_per_level() * CHUNK_DEPTH
    }

    fn total_bits() -> usize {
        Self::bits_per_level() * (CHUNK_DEPTH + CHUNK_PATH_LEN)
    }

    fn bits_per_level() -> usize {
        const { assert!(K.is_power_of_two(), "arity must be a power of two") };
        K.trailing_zeros() as usize
    }
}

impl<const CHUNK_DEPTH: usize, const CHUNK_PATH_LEN: usize, const K: usize, const CHUNK_SIZE: usize>
    AllocVar<ChunkInsertionProof<K, CHUNK_SIZE, CHUNK_PATH_LEN>, Fr>
    for ChunkInsertionProofVar<CHUNK_DEPTH, CHUNK_PATH_LEN, K, CHUNK_SIZE>
{
    fn new_variable<T: Borrow<ChunkInsertionProof<K, CHUNK_SIZE, CHUNK_PATH_LEN>>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        const {
            assert!(
                CHUNK_SIZE == K.pow(CHUNK_DEPTH as u32),
                "CHUNK_SIZE must equal K^CHUNK_DEPTH"
            )
        };

        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let existing_leaves =
            try_array_from_fn(|i| variable(cs.clone(), value.existing_leaves[i], mode))?;
        let new_leaves = try_array_from_fn(|i| variable(cs.clone(), value.new_leaves[i], mode))?;
        let current_siblings = try_array_from_fn(|i| {
            try_array_from_fn(|j| variable(cs.clone(), value.current_siblings[i][j], mode))
        })?;
        let next_siblings = try_array_from_fn(|i| {
            try_array_from_fn(|j| variable(cs.clone(), value.next_siblings[i][j], mode))
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
    use ark_relations::gr1cs::ConstraintSystem;

    use super::*;
    use crate::indexer::merkle_tree::IncrementalMerkleTree;

    const D: usize = 6;
    const K: usize = 2;
    const CHUNK_DEPTH: usize = 2;
    const CHUNK_SIZE: usize = 4;
    const CHUNK_PATH_LEN: usize = 4;

    /// Builds a tree pre-filled with `prefill` leaves, inserts `new_leaves`
    /// as a chunk batch, and runs the gadget end-to-end. `corrupt` flips a
    /// sibling to test that a tampered witness is rejected. Returns whether
    /// the resulting constraint system is satisfied, and (when satisfied)
    /// asserts the computed root matches the tree's real new root.
    fn run(prefill: usize, new_leaves: &[u64], corrupt: bool) -> bool {
        let hasher = crate::circuit::poseidon::PoseidonHasher::<K>::new().unwrap();
        let mut tree = IncrementalMerkleTree::<D, K>::new(hasher);
        if prefill > 0 {
            let leaves: Vec<Fr> = (0..prefill).map(|i| Fr::from(100 + i as u64)).collect();
            tree.append(&leaves);
        }

        let old_root = tree.root();
        let new_leaves_fr: Vec<Fr> = new_leaves.iter().map(|&v| Fr::from(v)).collect();
        let (mut proof, old_root_length) =
            tree.insert_chunk::<CHUNK_SIZE, CHUNK_PATH_LEN>(&new_leaves_fr);
        let new_root = tree.root();

        if corrupt {
            proof.existing_leaves[0] += Fr::from(1u64);
        }

        let cs = ConstraintSystem::<Fr>::new_ref();
        let proof_var =
            ChunkInsertionProofVar::<CHUNK_DEPTH, CHUNK_PATH_LEN, K, CHUNK_SIZE>::new_variable(
                cs.clone(),
                || Ok(&proof),
                AllocationMode::Witness,
            )
            .unwrap();
        let old_root_var = variable(cs.clone(), old_root, AllocationMode::Input).unwrap();
        let old_root_length_var = variable(
            cs.clone(),
            Fr::from(old_root_length as u64),
            AllocationMode::Input,
        )
        .unwrap();
        let native_hasher = crate::circuit::poseidon::PoseidonHasher::<K>::new().unwrap();
        let hasher_var = PoseidonHasherGadget::<K>::new_variable(
            cs.clone(),
            || Ok(&native_hasher),
            AllocationMode::Constant,
        )
        .unwrap();

        let computed_root = proof_var
            .insert(&old_root_var, &old_root_length_var, &hasher_var)
            .unwrap();

        let satisfied = cs.is_satisfied().unwrap();
        if satisfied {
            assert_eq!(computed_root.value().unwrap(), new_root);
        }
        satisfied
    }

    #[test]
    fn exact_fit_no_overflow() {
        // chunk_filled=1, +2 new leaves = 3 < CHUNK_SIZE=4, no overflow.
        assert!(run(1, &[10, 20], false));
    }

    #[test]
    fn overflow_into_next_chunk() {
        // chunk_filled=2, +3 new leaves = 5 > CHUNK_SIZE=4, overflows by 1.
        assert!(run(2, &[10, 20, 30], false));
    }

    #[test]
    fn starts_from_empty_chunk() {
        assert!(run(0, &[10, 20], false));
    }

    #[test]
    fn exact_boundary_fill() {
        // chunk_filled=1, +3 new leaves = 4 == CHUNK_SIZE exactly, no overflow.
        assert!(run(1, &[10, 20, 30], false));
    }

    #[test]
    fn negative_corrupted_sibling_rejected() {
        assert!(!run(1, &[10, 20], true));
    }
}

use ark_bn254::Fr;
use ark_ff::Zero;
use serde::{Deserialize, Serialize};

use crate::circuit::poseidon2::poseidon2_compress;

/// Fixed-depth incremental Merkle tree.
#[derive(Clone, Debug, Default)]
pub struct IncrementalMerkleTree<const D: usize, const K: usize> {
    levels: Vec<Vec<Fr>>,
    zeros: Vec<Fr>,
}

#[derive(Debug, thiserror::Error)]
pub enum MerkleTreeError {
    #[error("cannot insert more than SUBTREE_SIZE leaves at once")]
    TooManyLeaves,
}

/// Path from the root to a node in the Merkle tree, as one base-`K` digit
/// (0..K) per level.
pub type Path<const PATH_LEN: usize> = [u8; PATH_LEN];

/// Inclusion proof for a Merkle tree of depth `D` and arity `K`.
#[derive(Clone)]
pub struct InclusionProof<const PATH_LEN: usize, const K: usize> {
    pub path: Path<PATH_LEN>,
    pub siblings: [[Fr; K]; PATH_LEN],
    pub leaf: Fr,
}

/// Proof for appending up to `SUBTREE_SIZE` leaves into a Merkle tree of
/// depth `D` and arity `K`.
#[derive(Clone)]
pub struct SubtreeAppendProof<
    // Number of levels from the root to the subtree being appended.
    const SUBTREE_PATH_LEN: usize,
    // Number of leaves in the subtree being appended.
    const SUBTREE_SIZE: usize,
    // Arity of the Merkle tree.
    const K: usize,
> {
    /// The leaves in the subtree before this append, padded with zeros to `SUBTREE_SIZE`.
    pub existing_leaves: [Fr; SUBTREE_SIZE],
    /// The leaves being appended, padded with zeros to `SUBTREE_SIZE`.
    pub new_leaves: [Fr; SUBTREE_SIZE],
    /// The sibling hashes along the path to the current subtree.
    pub current_siblings: [[Fr; K]; SUBTREE_PATH_LEN],
    /// The sibling hashes along the path to the next subtree.
    pub next_siblings: [[Fr; K]; SUBTREE_PATH_LEN],
}

impl<const PATH_LEN: usize, const K: usize> InclusionProof<PATH_LEN, K> {
    /// Computes the Merkle root implied by this proof.
    pub fn root(&self) -> Fr {
        let mut current = self.leaf;
        for (&digit, sibling_group) in self.path.iter().rev().zip(self.siblings.iter().rev()) {
            let mut input = *sibling_group;
            input[digit as usize] = current;
            current = poseidon2_compress(&input);
        }
        current
    }
}

impl<const D: usize, const K: usize> IncrementalMerkleTree<D, K> {
    pub fn new() -> Self {
        let zeros = Self::compute_zeros();
        Self {
            levels: vec![Vec::new(); D + 1],
            zeros,
        }
    }

    pub fn from_leaves(leaves: &[Fr]) -> Self {
        let mut tree = Self::new();
        tree.append(leaves);
        tree
    }

    pub fn root(&self) -> Fr {
        self.levels[D].first().copied().unwrap_or(self.zeros[D])
    }

    pub fn len(&self) -> usize {
        self.levels[0].len()
    }

    /// Returns the path from the root to the given leaf if it exists in the tree.
    pub fn path(&self, leaf: Fr) -> Option<Path<D>> {
        let index = self.levels[0].iter().position(|&l| l == leaf)?;
        Some(Self::path_for_index(index))
    }

    /// Returns the inclusion proof for the node at the given path.
    pub fn inclusion<const PATH_LEN: usize>(
        &self,
        path: Path<PATH_LEN>,
    ) -> InclusionProof<PATH_LEN, K> {
        let level = const {
            assert!(PATH_LEN <= D, "PATH_LEN must be <= D");
            D - PATH_LEN
        };

        let mut index = Self::index_for_path(&path);
        let leaf = self.levels[level]
            .get(index)
            .copied()
            .unwrap_or(self.zeros[level]);

        let mut siblings = [[Fr::zero(); K]; PATH_LEN];
        for m in 0..PATH_LEN {
            let cur_level = level + m;
            let group_start = (index / K) * K;
            for k in 0..K {
                siblings[PATH_LEN - 1 - m][k] = self.levels[cur_level]
                    .get(group_start + k)
                    .copied()
                    .unwrap_or(self.zeros[cur_level]);
            }
            index /= K;
        }

        InclusionProof {
            path,
            siblings,
            leaf,
        }
    }

    /// Appends a list of leaves to the Merkle tree.
    pub fn append(&mut self, leaves: &[Fr]) {
        let start = self.levels[0].len();
        self.levels[0].extend_from_slice(leaves);
        for i in 0..leaves.len() {
            self.propagate(start + i);
        }
    }

    /// Inserts a leaf at the given index in the Merkle tree.
    pub fn insert(&mut self, index: usize, leaf: Fr) {
        if index >= self.levels[0].len() {
            self.levels[0].resize(index + 1, Fr::zero());
        }
        self.levels[0][index] = leaf;
        self.propagate(index);
    }

    /// Appends up to `SUBTREE_SIZE` leaves as a batch.
    ///
    /// Returns the subtree append proof or an error if `new_leaves.len() > SUBTREE_SIZE`.
    pub fn append_subtree<const SUBTREE_PATH_LEN: usize, const SUBTREE_SIZE: usize>(
        &mut self,
        new_leaves: &[Fr],
    ) -> Result<SubtreeAppendProof<SUBTREE_PATH_LEN, SUBTREE_SIZE, K>, MerkleTreeError> {
        const {
            assert!(
                SUBTREE_SIZE == K.pow((D - SUBTREE_PATH_LEN) as u32),
                "SUBTREE_SIZE must equal K^(D - SUBTREE_PATH_LEN)"
            );
        }
        if new_leaves.len() > SUBTREE_SIZE {
            return Err(MerkleTreeError::TooManyLeaves);
        }

        let old_root_length = self.levels[0].len();
        let subtree_index = old_root_length / SUBTREE_SIZE;
        let filled = old_root_length % SUBTREE_SIZE;

        let mut existing_leaves = [Fr::zero(); SUBTREE_SIZE];
        for (i, leaf) in existing_leaves.iter_mut().enumerate().take(filled) {
            *leaf = self.levels[0][subtree_index * SUBTREE_SIZE + i];
        }

        let mut padded_new_leaves = [Fr::zero(); SUBTREE_SIZE];
        padded_new_leaves[..new_leaves.len()].copy_from_slice(new_leaves);

        let current_siblings = self
            .inclusion::<SUBTREE_PATH_LEN>(Self::path_for_index(subtree_index))
            .siblings;
        for (i, &leaf) in new_leaves.iter().enumerate() {
            self.insert(old_root_length + i, leaf);
        }

        let next_siblings = self
            .inclusion::<SUBTREE_PATH_LEN>(Self::path_for_index(subtree_index + 1))
            .siblings;

        Ok(SubtreeAppendProof {
            existing_leaves,
            new_leaves: padded_new_leaves,
            current_siblings,
            next_siblings,
        })
    }

    /// Recomputes the ancestor chain of `levels[0][index]`, from its parent up
    /// to the root.
    fn propagate(&mut self, mut index: usize) {
        for level in 0..D {
            let group_start = (index / K) * K;
            let mut chunk = [self.zeros[level]; K];
            for (k, slot) in chunk.iter_mut().enumerate() {
                if let Some(&value) = self.levels[level].get(group_start + k) {
                    *slot = value;
                }
            }
            let parent_hash = poseidon2_compress(&chunk);

            let parent_index = index / K;
            if parent_index >= self.levels[level + 1].len() {
                self.levels[level + 1].resize(parent_index + 1, self.zeros[level + 1]);
            }
            self.levels[level + 1][parent_index] = parent_hash;

            index = parent_index;
        }
    }

    /// Decomposes `index` into a root-first, base-`K` path of digits.
    fn path_for_index<const PATH_LEN: usize>(mut index: usize) -> Path<PATH_LEN> {
        let mut path = [0u8; PATH_LEN];
        for m in 0..PATH_LEN {
            let digit = index % K;
            path[PATH_LEN - 1 - m] = digit as u8;
            index /= K;
        }
        path
    }

    /// Inverse of [`Self::path_for_index`].
    fn index_for_path<const PATH_LEN: usize>(path: &Path<PATH_LEN>) -> usize {
        let mut index = 0;
        for &digit in path.iter() {
            index = index * K + digit as usize;
        }
        index
    }

    /// The hash of an entirely-empty subtree at each level, `zeros[0] = 0`.
    fn compute_zeros() -> Vec<Fr> {
        let mut zeros = Vec::with_capacity(D + 1);
        zeros.push(Fr::zero());
        for _ in 0..D {
            let prev = *zeros.last().unwrap();
            zeros.push(poseidon2_compress(&[prev; K]));
        }
        zeros
    }
}

impl<const PATH_LEN: usize, const K: usize> Default for InclusionProof<PATH_LEN, K> {
    fn default() -> Self {
        InclusionProof {
            path: [0u8; PATH_LEN],
            siblings: [[Fr::zero(); K]; PATH_LEN],
            leaf: Fr::zero(),
        }
    }
}

impl<const SUBTREE_PATH_LEN: usize, const SUBTREE_SIZE: usize, const K: usize> Default
    for SubtreeAppendProof<SUBTREE_PATH_LEN, SUBTREE_SIZE, K>
{
    fn default() -> Self {
        SubtreeAppendProof {
            existing_leaves: [Fr::zero(); SUBTREE_SIZE],
            new_leaves: [Fr::zero(); SUBTREE_SIZE],
            current_siblings: [[Fr::zero(); K]; SUBTREE_PATH_LEN],
            next_siblings: [[Fr::zero(); K]; SUBTREE_PATH_LEN],
        }
    }
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize)]
struct IMTLeaves {
    #[serde_as(as = "Vec<crate::serde::fr::FrAsBytes>")]
    pub leaves: Vec<Fr>,
}

impl<const D: usize, const K: usize> Serialize for IncrementalMerkleTree<D, K> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let leaves = &self.levels[0];
        let leaves_wrapper = IMTLeaves {
            leaves: leaves.clone(),
        };
        leaves_wrapper.serialize(serializer)
    }
}

impl<'de, const D: usize, const K: usize> Deserialize<'de> for IncrementalMerkleTree<D, K> {
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: serde::Deserializer<'de>,
    {
        let leaves_wrapper = IMTLeaves::deserialize(deserializer)?;
        let tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves_wrapper.leaves);
        Ok(tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree_root_matches_zero_chain() {
        const D: usize = 3;
        const K: usize = 2;

        let tree = IncrementalMerkleTree::<D, K>::new();

        let mut expected = Fr::zero();
        for _ in 0..D {
            expected = poseidon2_compress(&[expected; K]);
        }

        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn root_matches_manual_computation() {
        const D: usize = 1;
        const K: usize = 2;

        let leaves = [Fr::from(11u64), Fr::from(22u64)];
        let tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves);

        assert_eq!(tree.root(), poseidon2_compress(&leaves));
    }

    #[test]
    fn leaf_inclusion_proof_round_trips() {
        const D: usize = 3;
        const K: usize = 2;

        let leaves: Vec<Fr> = (0..(1 << D)).map(|i| Fr::from(i as u64 + 1)).collect();
        let tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves);

        let target = leaves[5];
        let path = tree.path(target).expect("leaf should be found");
        let proof = tree.inclusion(path);

        assert_eq!(proof.leaf, target);
        assert_eq!(proof.root(), tree.root());
    }

    #[test]
    fn serialize_deserialize_imt() {
        const D: usize = 3;
        const K: usize = 2;

        let leaves: Vec<Fr> = (0..(1 << D)).map(|i| Fr::from(i as u64 + 1)).collect();
        let tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves);

        let serialized = postcard::to_stdvec(&tree).expect("serialization should succeed");
        let deserialized: IncrementalMerkleTree<D, K> =
            postcard::from_bytes(&serialized).expect("deserialization should succeed");

        assert_eq!(tree.root(), deserialized.root());
    }

    // TODO: Add tests for append_subtree
}

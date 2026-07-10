use ark_bn254::Fr;
use ark_ff::Zero;

use crate::circuit::poseidon::PoseidonHasher;

/// Fixed-depth incremental Merkle tree.
///
/// `levels[0]` holds the leaves and `levels[D]` always holds exactly the
/// root (once written). Inserting/appending only recomputes the ancestor
/// chain of the affected leaf (see [`Self::propagate`]), rather than
/// rebuilding the whole tree, since insertion is the common-case operation.
pub struct IncrementalMerkleTree<const D: usize, const K: usize> {
    hasher: PoseidonHasher<K>,
    levels: Vec<Vec<Fr>>,
    /// `zeros[l]` is the hash of an entirely-empty subtree at level `l`.
    zeros: Vec<Fr>,
}

/// Path from the root to a node in the Merkle tree, as one base-`K` digit
/// (0..K) per level.
pub type Path<const PATH_LEN: usize> = [u8; PATH_LEN];

/// Inclusion proof for a Merkle tree of depth `D` and arity `K`.
pub struct InclusionProof<const K: usize, const PATH_LEN: usize> {
    pub path: Path<PATH_LEN>,
    pub siblings: [[Fr; K]; PATH_LEN],
    pub leaf: Fr,
}

/// Witness for proving a batch of up to `CHUNK_SIZE` leaves was inserted into
/// the tree, starting right after the last previously-filled leaf (may span
/// two adjacent `CHUNK_SIZE`-leaf chunks). `current_siblings`/`next_siblings`
/// are the sibling path (of length `CHUNK_PATH_LEN`) above the current and
/// next chunk, respectively; the chunk index and fill level are not stored
/// here since they're derived in-circuit from the tree's leaf count instead
/// (see `ChunkInsertionProofVar::insert`).
pub struct ChunkInsertionProof<const K: usize, const CHUNK_SIZE: usize, const CHUNK_PATH_LEN: usize>
{
    pub existing_leaves: [Fr; CHUNK_SIZE],
    pub new_leaves: [Fr; CHUNK_SIZE],
    pub current_siblings: [[Fr; K]; CHUNK_PATH_LEN],
    pub next_siblings: [[Fr; K]; CHUNK_PATH_LEN],
}

impl<const K: usize, const PATH_LEN: usize> InclusionProof<K, PATH_LEN> {
    /// Recomputes the Merkle root implied by this proof.
    pub fn root(&self, hasher: &PoseidonHasher<K>) -> Fr {
        let mut current = self.leaf;
        for (&digit, sibling_group) in self.path.iter().rev().zip(self.siblings.iter().rev()) {
            let mut input = *sibling_group;
            input[digit as usize] = current;
            current = hasher.hash(input);
        }
        current
    }
}

impl<const D: usize, const K: usize> IncrementalMerkleTree<D, K> {
    pub fn new(hasher: PoseidonHasher<K>) -> Self {
        let zeros = Self::compute_zeros(&hasher);
        Self {
            hasher,
            levels: vec![Vec::new(); D + 1],
            zeros,
        }
    }

    pub fn from_leaves(leaves: &[Fr], hasher: PoseidonHasher<K>) -> Self {
        let zeros = Self::compute_zeros(&hasher);
        let levels = Self::build_levels(leaves, &hasher, &zeros);
        Self {
            hasher,
            levels,
            zeros,
        }
    }

    pub fn root(&self) -> Fr {
        self.levels[D].first().copied().unwrap_or(self.zeros[D])
    }

    /// Returns the path from the root to the given leaf if it exists in the tree.
    pub fn path(&self, leaf: Fr) -> Option<Path<D>> {
        let index = self.levels[0].iter().position(|&l| l == leaf)?;
        Some(Self::path_for_index(index))
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

    /// Returns the Merkle inclusion proof for the node at the given path.
    pub fn inclusion<const PATH_LEN: usize>(
        &self,
        path: Path<PATH_LEN>,
    ) -> Option<InclusionProof<K, PATH_LEN>> {
        let level = D.checked_sub(PATH_LEN)?;

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

        Some(InclusionProof {
            path,
            siblings,
            leaf,
        })
    }

    /// Inserts up to `CHUNK_SIZE` leaves as a batch, starting right after the
    /// last previously-inserted leaf. Returns the witness data needed by
    /// `ChunkInsertionProofVar::insert`, plus the tree's leaf count *before*
    /// this call (from which the circuit derives chunk index/fill level).
    pub fn insert_chunk<const CHUNK_SIZE: usize, const CHUNK_PATH_LEN: usize>(
        &mut self,
        new_leaves: &[Fr],
    ) -> (ChunkInsertionProof<K, CHUNK_SIZE, CHUNK_PATH_LEN>, usize) {
        assert!(
            new_leaves.len() <= CHUNK_SIZE,
            "chunk insertion cannot exceed CHUNK_SIZE new leaves"
        );
        assert!(
            CHUNK_SIZE == K.pow((D - CHUNK_PATH_LEN) as u32),
            "CHUNK_SIZE must equal K^(D - CHUNK_PATH_LEN)"
        );

        let old_root_length = self.levels[0].len();
        let chunk_index = old_root_length / CHUNK_SIZE;
        let chunk_filled = old_root_length % CHUNK_SIZE;

        let mut existing_leaves = [Fr::zero(); CHUNK_SIZE];
        for (i, leaf) in existing_leaves.iter_mut().enumerate().take(chunk_filled) {
            *leaf = self.levels[0][chunk_index * CHUNK_SIZE + i];
        }

        let mut padded_new_leaves = [Fr::zero(); CHUNK_SIZE];
        padded_new_leaves[..new_leaves.len()].copy_from_slice(new_leaves);

        // Captured from the tree's state *before* any mutation. When the
        // current and next chunk are themselves tree-siblings at the lowest
        // chunk-tree level (e.g. chunk indices 0 and 1), `current_siblings`'
        // lowest-level entry *is* the next chunk's root — which must still
        // reflect its old (pre-overflow) value here, since it's reused for
        // both the old-root proof and the post-current-chunk-update
        // intermediate-root proof, neither of which have touched the next
        // chunk yet.
        let current_siblings = self
            .inclusion::<CHUNK_PATH_LEN>(Self::path_for_index(chunk_index))
            .expect("current chunk path should resolve to a valid subtree")
            .siblings;

        for (i, &leaf) in new_leaves.iter().enumerate() {
            self.insert(old_root_length + i, leaf);
        }

        // Captured *after* all mutations. This is safe even though it's used
        // for the (pre-next-chunk-write) intermediate-root proof too: a
        // sibling array never includes the node's own subtree, and the
        // current chunk (the only other thing that changed) is only written
        // once, so its root is identical in the intermediate and final tree.
        let next_siblings = self
            .inclusion::<CHUNK_PATH_LEN>(Self::path_for_index(chunk_index + 1))
            .expect("next chunk path should resolve to a valid subtree")
            .siblings;

        (
            ChunkInsertionProof {
                existing_leaves,
                new_leaves: padded_new_leaves,
                current_siblings,
                next_siblings,
            },
            old_root_length,
        )
    }

    /// Recomputes the ancestor chain of `levels[0][index]`, from its parent up
    /// to the root. Touches exactly `D` nodes, regardless of tree size.
    fn propagate(&mut self, mut index: usize) {
        for level in 0..D {
            let group_start = (index / K) * K;
            let mut chunk = [self.zeros[level]; K];
            for (k, slot) in chunk.iter_mut().enumerate() {
                if let Some(&value) = self.levels[level].get(group_start + k) {
                    *slot = value;
                }
            }
            let parent_hash = self.hasher.hash(chunk);

            let parent_index = index / K;
            if parent_index >= self.levels[level + 1].len() {
                self.levels[level + 1].resize(parent_index + 1, self.zeros[level + 1]);
            }
            self.levels[level + 1][parent_index] = parent_hash;

            index = parent_index;
        }
    }

    /// The hash of an entirely-empty subtree at each level, `zeros[0] = 0`.
    fn compute_zeros(hasher: &PoseidonHasher<K>) -> Vec<Fr> {
        let mut zeros = Vec::with_capacity(D + 1);
        zeros.push(Fr::zero());
        for _ in 0..D {
            let prev = *zeros.last().unwrap();
            zeros.push(hasher.hash([prev; K]));
        }
        zeros
    }

    /// Builds the tree bottom-up from `leaves` in one pass. Used only for bulk
    /// construction; incremental updates go through [`Self::propagate`] instead.
    fn build_levels(leaves: &[Fr], hasher: &PoseidonHasher<K>, zeros: &[Fr]) -> Vec<Vec<Fr>> {
        let mut levels = Vec::with_capacity(D + 1);
        levels.push(leaves.to_vec());

        for level in 0..D {
            let cur = &levels[level];
            let mut next = Vec::with_capacity(cur.len().div_ceil(K));
            let mut i = 0;
            while i < cur.len() {
                let mut chunk = [zeros[level]; K];
                for (k, slot) in chunk.iter_mut().enumerate() {
                    if let Some(&value) = cur.get(i + k) {
                        *slot = value;
                    }
                }
                next.push(hasher.hash(chunk));
                i += K;
            }
            levels.push(next);
        }

        levels
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree_root_matches_zero_chain() {
        const D: usize = 3;
        const K: usize = 2;

        let hasher = PoseidonHasher::<K>::new().unwrap();
        let tree = IncrementalMerkleTree::<D, K>::new(hasher);

        let expect_hasher = PoseidonHasher::<K>::new().unwrap();
        let mut expected = Fr::zero();
        for _ in 0..D {
            expected = expect_hasher.hash([expected; K]);
        }

        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn root_matches_manual_computation() {
        const D: usize = 1;
        const K: usize = 2;

        let hasher = PoseidonHasher::<K>::new().unwrap();
        let leaves = [Fr::from(11u64), Fr::from(22u64)];
        let tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves, hasher);

        let verify_hasher = PoseidonHasher::<K>::new().unwrap();
        assert_eq!(tree.root(), verify_hasher.hash(leaves));
    }

    #[test]
    fn leaf_inclusion_proof_round_trips() {
        const D: usize = 3;
        const K: usize = 2;

        let hasher = PoseidonHasher::<K>::new().unwrap();
        let leaves: Vec<Fr> = (0..(1 << D)).map(|i| Fr::from(i as u64 + 1)).collect();
        let tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves, hasher);

        let target = leaves[5];
        let path = tree.path(target).expect("leaf should be found");
        let proof = tree.inclusion(path).expect("proof should be constructed");

        assert_eq!(proof.leaf, target);

        let verify_hasher = PoseidonHasher::<K>::new().unwrap();
        assert_eq!(proof.root(&verify_hasher), tree.root());
    }

    #[test]
    fn subtree_inclusion_proof_proves_subtree_root() {
        const D: usize = 3;
        const K: usize = 2;
        const PATH_LEN: usize = 2;
        const SUBTREE_INDEX: usize = 1;

        let hasher = PoseidonHasher::<K>::new().unwrap();
        let leaves: Vec<Fr> = (0..(1 << D)).map(|i| Fr::from(i as u64 + 1)).collect();
        let tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves, hasher);

        let path: Path<PATH_LEN> = IncrementalMerkleTree::<D, K>::path_for_index(SUBTREE_INDEX);
        let proof = tree
            .inclusion(path)
            .expect("subtree proof should be constructed");

        let verify_hasher = PoseidonHasher::<K>::new().unwrap();
        assert_eq!(proof.root(&verify_hasher), tree.root());

        // The subtree at `level = D - PATH_LEN` covers a contiguous slice of
        // `K^PATH_LEN` leaves; its root should independently match `proof.leaf`.
        let subtree_leaves = &leaves[SUBTREE_INDEX * K..(SUBTREE_INDEX + 1) * K];
        let subtree_hasher = PoseidonHasher::<K>::new().unwrap();
        let subtree_tree =
            IncrementalMerkleTree::<1, K>::from_leaves(subtree_leaves, subtree_hasher);

        assert_eq!(proof.leaf, subtree_tree.root());
    }

    #[test]
    fn incremental_append_matches_bulk_from_leaves() {
        const D: usize = 3;
        const K: usize = 2;

        let leaves: Vec<Fr> = (0..(1 << D)).map(|i| Fr::from(i as u64 + 1)).collect();

        let bulk_hasher = PoseidonHasher::<K>::new().unwrap();
        let bulk_tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves, bulk_hasher);

        let incremental_hasher = PoseidonHasher::<K>::new().unwrap();
        let mut incremental_tree = IncrementalMerkleTree::<D, K>::new(incremental_hasher);
        for leaf in &leaves {
            incremental_tree.append(std::slice::from_ref(leaf));
        }

        assert_eq!(incremental_tree.root(), bulk_tree.root());
    }

    #[test]
    fn insert_updates_root_and_remains_provable() {
        const D: usize = 3;
        const K: usize = 2;

        let hasher = PoseidonHasher::<K>::new().unwrap();
        let leaves: Vec<Fr> = (0..(1 << D)).map(|i| Fr::from(i as u64 + 1)).collect();
        let mut tree = IncrementalMerkleTree::<D, K>::from_leaves(&leaves, hasher);
        let root_before = tree.root();

        let updated_leaf = Fr::from(999u64);
        tree.insert(3, updated_leaf);

        assert_ne!(tree.root(), root_before);

        let path = tree
            .path(updated_leaf)
            .expect("updated leaf should be found");
        let proof = tree.inclusion(path).expect("proof should be constructed");
        assert_eq!(proof.leaf, updated_leaf);

        let verify_hasher = PoseidonHasher::<K>::new().unwrap();
        assert_eq!(proof.root(&verify_hasher), tree.root());
    }

    #[test]
    fn insert_chunk_matches_manual_insert() {
        const D: usize = 6;
        const K: usize = 2;
        const CHUNK_SIZE: usize = 4;
        const CHUNK_PATH_LEN: usize = 4;

        let hasher = PoseidonHasher::<K>::new().unwrap();
        let mut tree = IncrementalMerkleTree::<D, K>::new(hasher);
        // Pre-fill part of the first chunk manually, matching insert_chunk's
        // own semantics (contiguous append from leaf count 0).
        tree.append(&[Fr::from(1u64), Fr::from(2u64)]);

        let mut reference_tree =
            IncrementalMerkleTree::<D, K>::new(PoseidonHasher::<K>::new().unwrap());
        reference_tree.append(&[Fr::from(1u64), Fr::from(2u64)]);

        let old_root = tree.root();
        let new_leaves = [Fr::from(3u64), Fr::from(4u64), Fr::from(5u64)];
        let (proof, old_root_length) = tree.insert_chunk::<CHUNK_SIZE, CHUNK_PATH_LEN>(&new_leaves);

        assert_eq!(old_root_length, 2);
        assert_eq!(proof.existing_leaves[0], Fr::from(1u64));
        assert_eq!(proof.existing_leaves[1], Fr::from(2u64));
        assert_eq!(proof.existing_leaves[2], Fr::zero());
        assert_eq!(&proof.new_leaves[..3], &new_leaves);
        assert_eq!(proof.new_leaves[3], Fr::zero());

        reference_tree.append(&new_leaves);
        assert_ne!(tree.root(), old_root);
        assert_eq!(tree.root(), reference_tree.root());
    }
}

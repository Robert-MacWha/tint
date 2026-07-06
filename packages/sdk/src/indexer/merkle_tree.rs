use alloy::primitives::keccak256;

/// Standard fixed-depth incremental Merkle tree.
///
/// Uses append-only sequential insertion with a frontier (filled-subtrees) array.
/// Compatible with the MerkleTreeBatchInsert / MerkleTreeRootFromFrontier circuits.
///
/// TODO: replace hash_pair with Poseidon2-BN254 for production use.
pub struct IncrementalMerkleTree<const DEPTH: usize> {
    frontier: [[u8; 32]; DEPTH],
    zeros: [[u8; 32]; DEPTH],
    root: [u8; 32],
    size: usize,
    leaves: Vec<[u8; 32]>,
}

impl<const DEPTH: usize> IncrementalMerkleTree<DEPTH> {
    pub fn new() -> Self {
        let zeros = compute_zeros::<DEPTH>();
        let root = root_from_frontier(&[[0u8; 32]; DEPTH], 0, &zeros);
        Self {
            frontier: [[0u8; 32]; DEPTH],
            zeros,
            root,
            size: 0,
            leaves: Vec::new(),
        }
    }

    pub fn insert(&mut self, leaf: [u8; 32]) {
        self.leaves.push(leaf);
        let index = self.size;
        let mut current = leaf;

        for l in 0..DEPTH {
            if (index >> l) & 1 == 0 {
                self.frontier[l] = current;
                current = hash_pair(current, self.zeros[l]);
            } else {
                current = hash_pair(self.frontier[l], current);
            }
        }

        self.root = current;
        self.size += 1;
    }

    pub fn insert_many(&mut self, leaves: &[[u8; 32]]) {
        for &leaf in leaves {
            self.insert(leaf);
        }
    }

    pub fn root(&self) -> [u8; 32] {
        self.root
    }

    pub fn frontier(&self) -> [[u8; 32]; DEPTH] {
        self.frontier
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the inclusion proof siblings for the leaf at `leaf_index`.
    /// Bit `l` of `leaf_index` encodes the path: 0 = left child, 1 = right child at level `l`.
    /// This matches the `leafIndex` input to `MerkleTreeInclusion` in the circuit.
    pub fn prove(&self, leaf_index: u64) -> Option<[[u8; 32]; DEPTH]> {
        if leaf_index as usize >= self.size {
            return None;
        }
        let mut siblings = [[0u8; 32]; DEPTH];
        for l in 0..DEPTH {
            let sibling_pos = ((leaf_index >> l) ^ 1) as usize;
            siblings[l] = self.subtree_hash(l, sibling_pos);
        }
        Some(siblings)
    }

    fn subtree_hash(&self, level: usize, pos: usize) -> [u8; 32] {
        if pos << level >= self.size {
            return self.zeros[level];
        }
        if level == 0 {
            return self.leaves[pos];
        }
        hash_pair(
            self.subtree_hash(level - 1, pos * 2),
            self.subtree_hash(level - 1, pos * 2 + 1),
        )
    }
}

/// Compute zero hashes: zeros[l] = hash of an empty 2^l-leaf subtree.
fn compute_zeros<const DEPTH: usize>() -> [[u8; 32]; DEPTH] {
    let mut zeros = [[0u8; 32]; DEPTH];
    for l in 1..DEPTH {
        zeros[l] = hash_pair(zeros[l - 1], zeros[l - 1]);
    }
    zeros
}

/// Compute the root from a frontier and tree size without modifying the frontier.
pub fn root_from_frontier<const DEPTH: usize>(
    frontier: &[[u8; 32]; DEPTH],
    size: usize,
    zeros: &[[u8; 32]; DEPTH],
) -> [u8; 32] {
    let mut current = [0u8; 32];
    for l in 0..DEPTH {
        current = if (size >> l) & 1 == 0 {
            hash_pair(current, zeros[l])
        } else {
            hash_pair(frontier[l], current)
        };
    }
    current
}

/// Poseidon2 hash of two BN254 field elements.
///
/// TODO: replace with proper Poseidon2-BN254 implementation.
fn hash_pair(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(&a);
    data[32..].copy_from_slice(&b);
    keccak256(&data).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(n: u8) -> [u8; 32] {
        let mut v = [0u8; 32];
        v[31] = n;
        v
    }

    #[test]
    fn empty_tree_root_equals_root_from_empty_frontier() {
        let tree = IncrementalMerkleTree::<4>::new();
        let zeros = compute_zeros::<4>();
        let expected = root_from_frontier(&[[0u8; 32]; 4], 0, &zeros);
        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn root_from_frontier_matches_tree_root_after_insertions() {
        let mut tree = IncrementalMerkleTree::<8>::new();
        let zeros = compute_zeros::<8>();

        for i in 1u8..=10 {
            tree.insert(leaf(i));
            let recomputed = root_from_frontier(&tree.frontier(), tree.size(), &zeros);
            assert_eq!(tree.root(), recomputed, "mismatch after {} insertions", i);
        }
    }

    #[test]
    fn prove_reconstructs_root() {
        let mut tree = IncrementalMerkleTree::<8>::new();
        for i in 1u8..=10 {
            tree.insert(leaf(i));
        }

        for idx in 0u64..10 {
            let siblings = tree.prove(idx).expect("should have proof");
            let mut current = tree.leaves[idx as usize];
            for l in 0..8 {
                current = if (idx >> l) & 1 == 0 {
                    hash_pair(current, siblings[l])
                } else {
                    hash_pair(siblings[l], current)
                };
            }
            assert_eq!(current, tree.root(), "proof failed for leaf {}", idx);
        }

        assert!(tree.prove(10).is_none());
    }

    #[test]
    fn insert_many_matches_sequential_inserts() {
        let mut tree_a = IncrementalMerkleTree::<8>::new();
        let mut tree_b = IncrementalMerkleTree::<8>::new();
        let leaves: Vec<[u8; 32]> = (1u8..=5).map(leaf).collect();

        tree_a.insert_many(&leaves);
        for &l in &leaves {
            tree_b.insert(l);
        }

        assert_eq!(tree_a.root(), tree_b.root());
        assert_eq!(tree_a.frontier(), tree_b.frontier());
    }
}

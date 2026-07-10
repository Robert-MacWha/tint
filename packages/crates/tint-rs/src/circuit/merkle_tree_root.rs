use ark_relations::gr1cs::SynthesisError;

use crate::circuit::{FrVar, poseidon::PoseidonHasherGadget};

/// Computes the root of a Merkle tree given the leaves.
pub fn merkle_root<const D: usize, const K: usize, const LEAVES: usize>(
    leaves: &[FrVar; LEAVES],
    hasher: &PoseidonHasherGadget<K>,
) -> Result<FrVar, SynthesisError> {
    const {
        assert!(LEAVES == K.pow(D as u32), "LEAVES must be equal to K^D");
    }

    let mut current_hashes = leaves.to_vec();
    for _ in 0..D {
        let mut next_hashes = Vec::with_capacity(current_hashes.len() / K);
        for chunk in current_hashes.chunks(K) {
            let input = std::array::from_fn(|i| chunk[i].clone());
            let hash = hasher.hash(&input)?;
            next_hashes.push(hash);
        }
        current_hashes = next_hashes;
    }

    Ok(current_hashes[0].clone())
}

#[cfg(test)]
mod tests {
    use ark_bn254::Fr;
    use ark_ff::UniformRand;
    use ark_r1cs_std::{GR1CSVar, alloc::AllocVar};
    use ark_relations::gr1cs::ConstraintSystem;
    use ark_std::test_rng;

    use crate::circuit::poseidon::PoseidonHasher;

    use super::*;

    #[test]
    fn test_merkle_root() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut rng = test_rng();
        let native_hasher = PoseidonHasher::<2>::new().unwrap();
        let hasher = PoseidonHasherGadget::new_constant(cs.clone(), native_hasher.clone()).unwrap();

        let native_leaves: [Fr; 6] = std::array::from_fn(|_| Fr::rand(&mut rng));
        let leaves = std::array::from_fn(|i| {
            FrVar::new_witness(cs.clone(), || Ok(native_leaves[i])).unwrap()
        });

        let root = merkle_root::<2, 2, 4>(&leaves, &hasher)
            .unwrap()
            .value()
            .unwrap();

        let computed_root = native_hasher.hash([
            native_hasher.hash([native_leaves[0], native_leaves[1]]),
            native_hasher.hash([native_leaves[2], native_leaves[3]]),
        ]);

        assert_eq!(root, computed_root);
    }
}

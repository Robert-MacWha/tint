use ark_relations::gr1cs::SynthesisError;

use crate::circuit::{FrVar, poseidon::poseidon_hash_gadget};

/// Computes the root of a Merkle tree given the leaves.
#[tracing::instrument(target = "r1cs", skip_all)]
pub fn root_proof<const D: usize, const K: usize, const LEAVES: usize>(
    leaves: &[FrVar; LEAVES],
) -> Result<FrVar, SynthesisError> {
    const {
        assert!(LEAVES == K.pow(D as u32), "LEAVES must be equal to K^D");
    }

    let mut current_hashes = leaves.to_vec();
    for _ in 0..D {
        let mut next_hashes = Vec::with_capacity(current_hashes.len() / K);
        for chunk in current_hashes.chunks(K) {
            let input: [FrVar; K] = std::array::from_fn(|i| chunk[i].clone());
            let hash = poseidon_hash_gadget(&input)?;
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
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;
    use ark_std::test_rng;

    use crate::circuit::{poseidon::poseidon_hash, witness};

    use super::*;

    #[test]
    fn test_merkle_root() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut rng = test_rng();

        let native_leaves: [Fr; 6] = std::array::from_fn(|_| Fr::rand(&mut rng));
        let leaves = std::array::from_fn(|i| witness(cs.clone(), &native_leaves[i]).unwrap());

        let root = root_proof::<2, 2, 4>(&leaves).unwrap().value().unwrap();

        let computed_root = poseidon_hash(&[
            poseidon_hash(&[native_leaves[0], native_leaves[1]]),
            poseidon_hash(&[native_leaves[2], native_leaves[3]]),
        ]);

        assert_eq!(root, computed_root);
    }
}

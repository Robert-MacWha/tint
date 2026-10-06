use ark_bn254::Fr;

use crate::circuit::{
    FrVar,
    poseidon2::{poseidon2_compress, poseidon2_compress_gadget},
};

/// [`skew_mmr::hasher::Hasher<Fr>`] impl for Poseidon2.
#[derive(Debug, Clone)]
pub struct Poseidon2Hasher;

impl skew_mmr::hasher::Hasher<Fr> for Poseidon2Hasher {
    fn hash(element: &Fr, left: &Fr, right: &Fr) -> Fr {
        let state = [*element, *left, *right];
        poseidon2_compress(&state)
    }
}

impl skew_mmr::constraints::hasher::HasherGadget<Fr> for Poseidon2Hasher {
    fn hash(
        element: &FrVar,
        left: &FrVar,
        right: &FrVar,
    ) -> Result<FrVar, ark_relations::gr1cs::SynthesisError> {
        let state = [element.clone(), left.clone(), right.clone()];
        poseidon2_compress_gadget(&state)
    }
}

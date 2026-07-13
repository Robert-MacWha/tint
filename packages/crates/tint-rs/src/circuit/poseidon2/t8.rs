use ark_bn254::Fr;
use taceo_poseidon2::bn254::t8::POSEIDON2_BN254_T8_PARAMS;

use crate::circuit::poseidon2::element::PoseidonElement;

use super::common::{Width, matmul_m4};

pub struct T8;

impl Width<8> for T8 {
    const EXTERNAL: &'static [[Fr; 8]; 8] = &POSEIDON2_BN254_T8_PARAMS.round_constants_external;
    const INTERNAL: &'static [Fr] = &POSEIDON2_BN254_T8_PARAMS.round_constants_internal;
    const DIAG: &'static [Fr; 8] = &POSEIDON2_BN254_T8_PARAMS.mat_internal_diag_m_1;

    /// The external-round linear layer for `t=8`: an M4 mix within each half
    /// of the state, then a cheap combination across the two halves.
    fn matmul_external<E: PoseidonElement>(state: &mut [E; 8]) {
        let mut lo: [E; 4] = [
            state[0].clone(),
            state[1].clone(),
            state[2].clone(),
            state[3].clone(),
        ];
        let mut hi: [E; 4] = [
            state[4].clone(),
            state[5].clone(),
            state[6].clone(),
            state[7].clone(),
        ];
        matmul_m4(&mut lo);
        matmul_m4(&mut hi);

        let stored: [E; 4] = std::array::from_fn(|l| lo[l].clone() + hi[l].clone());

        for l in 0..4 {
            state[l] = lo[l].clone() + stored[l].clone();
            state[4 + l] = hi[l].clone() + stored[l].clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use ark_ff::UniformRand;
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;

    use crate::circuit::{
        FrVar,
        poseidon2::{poseidon2_compress, poseidon2_compress_gadget},
        witness,
    };

    use super::*;

    /// Test that our permutation matches taceo-poseidon2's own native
    /// permutation for `t=8`.
    #[test]
    fn test_matches_taceo_poseidon2_8() {
        let mut rng = ark_std::test_rng();
        let cs = ConstraintSystem::new_ref();

        for _ in 0..10 {
            let inputs: [Fr; 8] = core::array::from_fn(|_| Fr::rand(&mut rng));
            let inputs_var: [FrVar; 8] =
                core::array::from_fn(|i| witness(cs.clone(), &inputs[i]).unwrap());

            let reference = taceo_poseidon2::bn254::t8::permutation(&inputs)[0];
            let native_hash = poseidon2_compress(&inputs);
            let gadget_hash = poseidon2_compress_gadget(&inputs_var).unwrap();

            assert_eq!(reference, native_hash);
            assert_eq!(gadget_hash.value().unwrap(), native_hash);
        }

        assert!(cs.is_satisfied().unwrap());
    }
}

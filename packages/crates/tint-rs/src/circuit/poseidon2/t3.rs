use ark_bn254::Fr;
use taceo_poseidon2::bn254::t3::POSEIDON2_BN254_T3_PARAMS;

use crate::circuit::poseidon2::element::PoseidonElement;

use super::common::Width;

pub struct T3;

impl Width<3> for T3 {
    const EXTERNAL: &'static [[Fr; 3]; 8] = &POSEIDON2_BN254_T3_PARAMS.round_constants_external;
    const INTERNAL: &'static [Fr] = &POSEIDON2_BN254_T3_PARAMS.round_constants_internal;
    const DIAG: &'static [Fr; 3] = &POSEIDON2_BN254_T3_PARAMS.mat_internal_diag_m_1;

    /// The `circ(2, 1, 1)` external linear layer used for `t=3`.
    fn matmul_external<E: PoseidonElement>(state: &mut [E; 3]) {
        let sum = state[0].clone() + state[1].clone() + state[2].clone();
        state[0] = state[0].clone() + sum.clone();
        state[1] = state[1].clone() + sum.clone();
        state[2] = state[2].clone() + sum;
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
    /// permutation for `t=3`.
    #[test]
    fn test_matches_taceo_poseidon2_3() {
        let mut rng = ark_std::test_rng();
        let cs = ConstraintSystem::new_ref();

        for _ in 0..10 {
            let inputs: [Fr; 3] = core::array::from_fn(|_| Fr::rand(&mut rng));
            let inputs_var: [FrVar; 3] =
                core::array::from_fn(|i| witness(cs.clone(), &inputs[i]).unwrap());

            let reference = taceo_poseidon2::bn254::t3::permutation(&inputs)[0];
            let native_hash = poseidon2_compress(&inputs);
            let gadget_hash = poseidon2_compress_gadget(&inputs_var).unwrap();

            assert_eq!(reference, native_hash);
            assert_eq!(gadget_hash.value().unwrap(), native_hash);
        }

        assert!(cs.is_satisfied().unwrap());
    }
}

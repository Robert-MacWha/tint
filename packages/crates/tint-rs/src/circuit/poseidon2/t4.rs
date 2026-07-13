use ark_bn254::Fr;
use taceo_poseidon2::bn254::t4::POSEIDON2_BN254_T4_PARAMS;

use crate::circuit::poseidon2::element::PoseidonElement;

use super::common::{Width, matmul_m4};

pub struct T4;

impl Width<4> for T4 {
    const EXTERNAL: &'static [[Fr; 4]; 8] = &POSEIDON2_BN254_T4_PARAMS.round_constants_external;
    const INTERNAL: &'static [Fr] = &POSEIDON2_BN254_T4_PARAMS.round_constants_internal;
    const DIAG: &'static [Fr; 4] = &POSEIDON2_BN254_T4_PARAMS.mat_internal_diag_m_1;

    /// The external-round linear layer for `t=4`: a single M4 mix.
    fn matmul_external<E: PoseidonElement>(state: &mut [E; 4]) {
        matmul_m4(state);
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
    /// permutation for `t=4`.
    #[test]
    fn test_matches_taceo_poseidon2_4() {
        let mut rng = ark_std::test_rng();
        let cs = ConstraintSystem::new_ref();

        for _ in 0..10 {
            let inputs: [Fr; 4] = core::array::from_fn(|_| Fr::rand(&mut rng));
            let inputs_var: [FrVar; 4] =
                core::array::from_fn(|i| witness(cs.clone(), &inputs[i]).unwrap());

            let reference = taceo_poseidon2::bn254::t4::permutation(&inputs)[0];
            let native_hash = poseidon2_compress(&inputs);
            let gadget_hash = poseidon2_compress_gadget(&inputs_var).unwrap();

            assert_eq!(reference, native_hash);
            assert_eq!(gadget_hash.value().unwrap(), native_hash);
        }

        assert!(cs.is_satisfied().unwrap());
    }
}

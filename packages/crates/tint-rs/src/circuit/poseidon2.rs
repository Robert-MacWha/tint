use ark_bn254::Fr;
use ark_relations::gr1cs::SynthesisError;
use taceo_poseidon2::bn254::t8::POSEIDON2_BN254_T8_PARAMS;

use crate::circuit::{
    FrVar,
    poseidon::{PoseidonElem, poseidon_hash, poseidon_hash_gadget},
};

const ALPHA: u64 = 5;

/// Compresses 8 field elements into a single field element using the
/// Poseidon2 permutation (bn254, `t=8`, no capacity slot). The output is a
/// single truncated element (index 0) of the permuted 8-element state.
pub fn poseidon2_compress_8(input: &[Fr; 8]) -> Fr {
    let mut state = *input;
    //? permute cannot fail for native field elements
    permute(&mut state).expect("Poseidon2 permutation failed");
    state[0]
}

#[tracing::instrument(target = "r1cs", skip_all, name = "poseidon2_compress_8")]
pub fn poseidon2_compress_8_gadget(input: &[FrVar; 8]) -> Result<FrVar, SynthesisError> {
    let mut state = input.clone();
    permute(&mut state)?;
    Ok(state[0].clone())
}

/// Hashes `K` Merkle-tree children into their parent, using the cheaper
/// Poseidon2 compression permutation when `K == 8` (this circuit's Merkle
/// arity) and the general Poseidon sponge otherwise.
pub fn hash_children<const K: usize>(input: &[Fr; K]) -> Fr {
    if K == 8 {
        let input8: [Fr; 8] = input
            .to_vec()
            .try_into()
            .unwrap_or_else(|_| unreachable!("K == 8 checked above"));
        poseidon2_compress_8(&input8)
    } else {
        poseidon_hash(input)
    }
}

/// In-circuit counterpart of [`hash_children`].
pub fn hash_children_gadget<const K: usize>(
    input: &[FrVar; K],
) -> Result<FrVar, SynthesisError> {
    if K == 8 {
        let input8: [FrVar; 8] = input
            .to_vec()
            .try_into()
            .unwrap_or_else(|_| unreachable!("K == 8 checked above"));
        poseidon2_compress_8_gadget(&input8)
    } else {
        poseidon_hash_gadget(input)
    }
}

/// Applies the Poseidon2 `t=8` permutation to `state` in place, over either
/// native field elements or their in-circuit counterparts.
fn permute<E: PoseidonElem + Clone>(state: &mut [E; 8]) -> Result<(), SynthesisError> {
    let params = &POSEIDON2_BN254_T8_PARAMS;

    matmul_external(state);

    for rc in &params.round_constants_external[..4] {
        external_round(state, rc)?;
    }
    for rc in params.round_constants_internal {
        internal_round(state, rc, &params.mat_internal_diag_m_1)?;
    }
    for rc in &params.round_constants_external[4..] {
        external_round(state, rc)?;
    }

    Ok(())
}

fn external_round<E: PoseidonElem + Clone>(
    state: &mut [E; 8],
    rc: &[Fr; 8],
) -> Result<(), SynthesisError> {
    for i in 0..8 {
        state[i].add_constant(rc[i]);
    }
    for elem in state.iter_mut() {
        *elem = elem.pow_alpha(ALPHA)?;
    }
    matmul_external(state);
    Ok(())
}

fn internal_round<E: PoseidonElem + Clone>(
    state: &mut [E; 8],
    rc: Fr,
    diag: &[Fr; 8],
) -> Result<(), SynthesisError> {
    state[0].add_constant(rc);
    state[0] = state[0].pow_alpha(ALPHA)?;
    matmul_internal(state, diag);
    Ok(())
}

/// The 4x4 "M4" MDS matrix multiplication used to build the Poseidon2
/// external linear layer, matching `taceo_poseidon2::perm::matmul_m4`.
fn matmul_m4<E: PoseidonElem + Clone>(input: &mut [E]) {
    let t0 = input[0].clone() + input[1].clone(); // A + B
    let t1 = input[2].clone() + input[3].clone(); // C + D
    let t2 = input[1].mul_constant(Fr::from(2u64)) + t1.clone(); // 2B + C + D
    let t3 = input[3].mul_constant(Fr::from(2u64)) + t0.clone(); // A + B + 2D
    let t4 = t1.mul_constant(Fr::from(4u64)) + t3.clone(); // A + B + 4C + 6D
    let t5 = t0.mul_constant(Fr::from(4u64)) + t2.clone(); // 4A + 6B + C + D
    let t6 = t3 + t5.clone(); // 5A + 7B + C + 3D
    let t7 = t2 + t4.clone(); // A + 3B + 5C + 7D
    input[0] = t6;
    input[1] = t5;
    input[2] = t7;
    input[3] = t4;
}

/// The external-round linear layer for `t=8`: an M4 mix within each half of
/// the state, then a cheap combination across the two halves.
fn matmul_external<E: PoseidonElem + Clone>(state: &mut [E; 8]) {
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

/// The internal-round linear layer: `state[i] = state[i] * diag[i] + sum(state)`.
fn matmul_internal<E: PoseidonElem + Clone>(state: &mut [E; 8], diag: &[Fr; 8]) {
    let sum = state
        .iter()
        .cloned()
        .reduce(|acc, term| acc + term)
        .expect("poseidon2 state is never empty");

    for i in 0..8 {
        state[i] = state[i].mul_constant(diag[i]) + sum.clone();
    }
}

#[cfg(test)]
mod tests {
    use ark_ff::UniformRand;
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;
    use ark_std::rand::Rng;

    use crate::circuit::witness;

    use super::*;

    /// Test that our generic permutation matches taceo-poseidon2's own
    /// native permutation for the same width (`t=8`).
    #[test]
    fn test_matches_taceo_poseidon2() {
        let mut rng = ark_std::test_rng();
        check_matches(10, &mut rng);
    }

    fn check_matches(n: usize, rng: &mut impl Rng) {
        let cs = ConstraintSystem::new_ref();

        for _ in 0..n {
            let inputs: [Fr; 8] = core::array::from_fn(|_| Fr::rand(rng));
            let inputs_var: [FrVar; 8] =
                core::array::from_fn(|i| witness(cs.clone(), &inputs[i]).unwrap());

            let reference = taceo_poseidon2::bn254::t8::permutation(&inputs)[0];
            let native_hash = poseidon2_compress_8(&inputs);
            let gadget_hash = poseidon2_compress_8_gadget(&inputs_var).unwrap();

            assert_eq!(
                reference, native_hash,
                "poseidon2_compress_8 does not match taceo-poseidon2's native permutation",
            );

            assert_eq!(
                gadget_hash.value().unwrap(),
                native_hash,
                "poseidon2_compress_8_gadget does not match poseidon2_compress_8",
            );
        }

        assert!(
            cs.is_satisfied().unwrap(),
            "poseidon2_compress_8_gadget constraints are not satisfied"
        );
    }
}

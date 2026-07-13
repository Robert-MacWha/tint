use ark_bn254::Fr;

use crate::circuit::poseidon2::element::PoseidonElement;

/// S-box exponent, shared by every Poseidon2 width.
const ALPHA: u64 = 5;

/// The width-`T`-specific parts of a Poseidon2 permutation.
pub trait Width<const T: usize> {
    const EXTERNAL: &'static [[Fr; T]; 8];
    const INTERNAL: &'static [Fr];
    const DIAG: &'static [Fr; T];

    fn matmul_external<E: PoseidonElement>(state: &mut [E; T]);
}

/// Applies a Poseidon2 permutation of width `T` to `state` in place.
pub fn permute<W: Width<T>, E: PoseidonElement, const T: usize>(
    state: &mut [E; T],
) -> Result<(), E::Error> {
    W::matmul_external(state);

    for rc in &W::EXTERNAL[..4] {
        external_round::<W, E, T>(state, rc)?;
    }
    for &rc in W::INTERNAL {
        internal_round::<W, E, T>(state, rc)?;
    }
    for rc in &W::EXTERNAL[4..] {
        external_round::<W, E, T>(state, rc)?;
    }

    Ok(())
}

/// The 4x4 "M4" MDS matrix multiplication, matching
/// `taceo_poseidon2::perm::matmul_m4`.
pub fn matmul_m4<E: PoseidonElement>(input: &mut [E]) {
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

fn external_round<W: Width<T>, E: PoseidonElement, const T: usize>(
    state: &mut [E; T],
    rc: &[Fr; T],
) -> Result<(), E::Error> {
    for i in 0..T {
        state[i].add_constant(rc[i]);
    }
    for elem in state.iter_mut() {
        *elem = elem.pow_alpha(ALPHA)?;
    }
    W::matmul_external(state);
    Ok(())
}

fn internal_round<W: Width<T>, E: PoseidonElement, const T: usize>(
    state: &mut [E; T],
    rc: Fr,
) -> Result<(), E::Error> {
    state[0].add_constant(rc);
    state[0] = state[0].pow_alpha(ALPHA)?;
    matmul_internal(state, W::DIAG);
    Ok(())
}

fn matmul_internal<E: PoseidonElement, const T: usize>(state: &mut [E; T], diag: &[Fr; T]) {
    let sum = state
        .iter()
        .cloned()
        .reduce(|acc, term| acc + term)
        .expect("poseidon2 state is never empty");

    for i in 0..T {
        state[i] = state[i].mul_constant(diag[i]) + sum.clone();
    }
}

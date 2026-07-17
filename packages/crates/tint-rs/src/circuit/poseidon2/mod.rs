mod common;
mod element;
mod t2;
mod t3;
// mod t4;
mod t8;

use ark_bn254::Fr;
use ark_relations::gr1cs::SynthesisError;

use crate::circuit::{FrVar, poseidon2::element::PoseidonElement};

/// Compresses `T` field elements into one using the Poseidon2 permutation.
/// Supported: `T` = 2, 3, 4, 8.
pub fn poseidon2_compress<const T: usize>(input: &[Fr; T]) -> Fr {
    let mut state = *input;
    //? permute cannot fail for native field elements
    permute(&mut state).expect("Poseidon2 permutation failed");

    // Feed-forward the first input (matches the taceo `Poseidon2T*_BN254._compress`).
    state[0] + input[0]
}

/// In-circuit counterpart of [`poseidon2_compress`].
#[tracing::instrument(target = "r1cs", skip_all, name = "poseidon2_compress")]
pub fn poseidon2_compress_gadget<const T: usize>(
    input: &[FrVar; T],
) -> Result<FrVar, SynthesisError> {
    let mut state = input.clone();
    permute(&mut state)?;

    // Feed-forward the first input (matches the taceo `Poseidon2T*_BN254._compress`).
    Ok(state[0].clone() + input[0].clone())
}

fn permute<E: PoseidonElement, const T: usize>(state: &mut [E; T]) -> Result<(), E::Error> {
    const {
        assert!(
            matches!(T, 2 | 3 | 8),
            "poseidon2: unsupported width (must be 2, 3, or 8)"
        )
    };

    match T {
        2 => common::permute::<t2::T2, E, 2>((&mut state[..]).try_into().unwrap()),
        3 => common::permute::<t3::T3, E, 3>((&mut state[..]).try_into().unwrap()),
        // 4 => common::permute::<t4::T4, E, 4>((&mut state[..]).try_into().unwrap()),
        8 => common::permute::<t8::T8, E, 8>((&mut state[..]).try_into().unwrap()),
        _ => unreachable!(),
    }
}

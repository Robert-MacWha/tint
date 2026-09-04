mod common;
pub mod crh;
mod element;
mod t2;
mod t3;
mod t8;

use ark_bn254::Fr;
use ark_relations::gr1cs::SynthesisError;

use crate::circuit::{FrVar, poseidon2::element::PoseidonElement};

/// Compresses `T` field elements into one using the Poseidon2 permutation.
/// Supported: `T` = 1, 2, 3, 8.
#[must_use]
pub fn poseidon2_compress<const T: usize>(input: &[Fr; T]) -> Fr {
    const {
        assert!(
            matches!(T, 1 | 2 | 3 | 8),
            "poseidon2: unsupported width (must be 1, 2, 3, or 8)"
        );
    };

    let mut state = *input;
    //? E::Error is infallible for native Fr
    permute(&mut state);

    // Feed-forward the first input (matches the taceo `Poseidon2T*_BN254._compress`).
    state[0] + input[0]
}

/// In-circuit counterpart of [`poseidon2_compress`].
#[tracing::instrument(target = "r1cs", skip_all, name = "poseidon2_compress")]
pub fn poseidon2_compress_gadget<const T: usize>(
    input: &[FrVar; T],
) -> Result<FrVar, SynthesisError> {
    const {
        assert!(
            matches!(T, 1 | 2 | 3 | 8),
            "poseidon2: unsupported width (must be 1, 2, 3, or 8)"
        );
    };

    let mut state = input.clone();
    permute(&mut state)?;

    // Feed-forward the first input (matches the taceo `Poseidon2T*_BN254._compress`).
    Ok(state[0].clone() + input[0].clone())
}

#[allow(clippy::unwrap_used, clippy::unreachable)]
fn permute<E: PoseidonElement, const T: usize>(state: &mut [E; T]) -> Result<(), E::Error> {
    const {
        assert!(
            matches!(T, 1 | 2 | 3 | 8),
            "poseidon2: unsupported width (must be 1, 2, 3, or 8)"
        );
    };

    // SAFETY: transmute is safe because the length of the slice is guaranteed to be `T`.
    // SAFETY: the `match` arms below are exhaustive for the supported values of `T`.
    match T {
        1 => {
            let mut buf = [state[0].clone(), E::zero()];
            common::permute::<t2::T2, E, 2>(&mut buf)?;
            state[0] = buf[0].clone();
            Ok(())
        }
        2 => common::permute::<t2::T2, E, 2>((&mut state[..]).try_into().unwrap()),
        3 => common::permute::<t3::T3, E, 3>((&mut state[..]).try_into().unwrap()),
        8 => common::permute::<t8::T8, E, 8>((&mut state[..]).try_into().unwrap()),
        _ => unreachable!(),
    }
}

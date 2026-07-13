mod common;
mod element;
mod t2;
mod t3;
mod t4;
mod t8;

use ark_bn254::Fr;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::gr1cs::SynthesisError;

use crate::circuit::{FrVar, poseidon2::element::PoseidonElement};

/// Compresses `T` field elements into one using the Poseidon2 permutation.
/// Supported: `T` = 2, 3, 4, 8.
pub fn poseidon2_compress<const T: usize>(input: &[Fr; T]) -> Fr {
    let mut state = *input;
    //? permute cannot fail for native field elements
    permute(&mut state).expect("Poseidon2 permutation failed");
    state[0]
}

/// In-circuit counterpart of [`poseidon2_compress`].
#[tracing::instrument(target = "r1cs", skip_all, name = "poseidon2_compress")]
pub fn poseidon2_compress_gadget<const T: usize>(
    input: &[FrVar; T],
) -> Result<FrVar, SynthesisError> {
    let mut state = input.clone();
    permute(&mut state)?;
    Ok(state[0].clone())
}

/// Hashes `N` field elements using the Poseidon2 `t=4` sponge matching
/// `poseidon2-evm`'s `LibPoseidon2Yul` `hash_1`/`hash_2`/`hash_3`.
/// Supported: `N` = 1, 2, 3.
pub fn poseidon2_hash<const N: usize>(input: &[Fr; N]) -> Fr {
    const { assert!(N >= 1 && N <= 3, "poseidon2_hash: N must be 1, 2, or 3") };

    let mut state = [Fr::from(0u64); 4];
    state[..N].copy_from_slice(input);
    state[3] = Fr::from((N as u128) << 64);
    match common::permute::<t4::T4, Fr, 4>(&mut state) {
        Ok(()) => (),
        //? permute cannot fail for native field elements
        Err(()) => panic!("Poseidon2 permutation failed"),
    }
    state[0]
}

/// In-circuit counterpart of [`poseidon2_hash`].
#[tracing::instrument(target = "r1cs", skip_all, name = "poseidon2_hash")]
pub fn poseidon2_hash_gadget<const N: usize>(input: &[FrVar; N]) -> Result<FrVar, SynthesisError> {
    const { assert!(N >= 1 && N <= 3, "poseidon2_hash: N must be 1, 2, or 3") };

    let mut state: [FrVar; 4] = std::array::from_fn(|_| FrVar::constant(Fr::from(0u64)));
    state[..N].clone_from_slice(input);
    state[3] = FrVar::constant(Fr::from((N as u128) << 64));
    common::permute::<t4::T4, FrVar, 4>(&mut state)?;
    Ok(state[0].clone())
}

fn permute<E: PoseidonElement, const T: usize>(state: &mut [E; T]) -> Result<(), E::Error> {
    const {
        assert!(
            matches!(T, 2 | 3 | 4 | 8),
            "poseidon2: unsupported width (must be 2, 3, 4, or 8)"
        )
    };

    match T {
        2 => common::permute::<t2::T2, E, 2>((&mut state[..]).try_into().unwrap()),
        3 => common::permute::<t3::T3, E, 3>((&mut state[..]).try_into().unwrap()),
        4 => common::permute::<t4::T4, E, 4>((&mut state[..]).try_into().unwrap()),
        8 => common::permute::<t8::T8, E, 8>((&mut state[..]).try_into().unwrap()),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use ark_ff::UniformRand;
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;

    use crate::circuit::witness;

    use super::*;

    /// Test that `poseidon2_hash`'s gadget matches its native counterpart,
    /// for `N = 1`, `2`, and `3`.
    #[test]
    fn test_poseidon2_hash_gadget_matches_native() {
        let mut rng = ark_std::test_rng();
        let cs = ConstraintSystem::new_ref();

        for _ in 0..10 {
            let [x, y, z]: [Fr; 3] = core::array::from_fn(|_| Fr::rand(&mut rng));
            let x_var: FrVar = witness(cs.clone(), &x).unwrap();
            let y_var: FrVar = witness(cs.clone(), &y).unwrap();
            let z_var: FrVar = witness(cs.clone(), &z).unwrap();

            let native_1 = poseidon2_hash(&[x]);
            let gadget_1 = poseidon2_hash_gadget(&[x_var.clone()]).unwrap();
            assert_eq!(gadget_1.value().unwrap(), native_1);

            let native_2 = poseidon2_hash(&[x, y]);
            let gadget_2 = poseidon2_hash_gadget(&[x_var.clone(), y_var.clone()]).unwrap();
            assert_eq!(gadget_2.value().unwrap(), native_2);

            let native_3 = poseidon2_hash(&[x, y, z]);
            let gadget_3 = poseidon2_hash_gadget(&[x_var, y_var, z_var]).unwrap();
            assert_eq!(gadget_3.value().unwrap(), native_3);
        }

        assert!(cs.is_satisfied().unwrap());
    }
}

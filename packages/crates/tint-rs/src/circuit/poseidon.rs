use std::{borrow::Borrow, ops::Add};

use ark_bn254::Fr;
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_ff::{Field, PrimeField, Zero};
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    fields::FieldVar,
};
use ark_relations::gr1cs::{Namespace, SynthesisError};
use std::sync::Arc;

use crate::circuit::FrVar;

/// A circom-compatible Poseidon hasher over `N` field elements.
///
/// Matches light-poseidon's `Poseidon` hash (circomlib) for on-chain
/// compatibility.
#[derive(Debug, Clone)]
pub struct PoseidonHasher<const N: usize> {
    pub parameters: Arc<PoseidonConfig<Fr>>,
}

/// In-circuit counterpart of [`PoseidonHasher`].
#[derive(Clone)]
pub struct PoseidonHasherGadget<const N: usize> {
    pub parameters: Arc<PoseidonConfig<Fr>>,
}

/// A native field element (`Fr`) or its in-circuit counterpart (`FrVar`).
trait PoseidonElem: Sized + Add<Self, Output = Self> {
    fn add_constant(&mut self, constant: Fr);
    fn pow_alpha(&self, alpha: u64) -> Result<Self, SynthesisError>;
    fn mul_constant(&self, constant: Fr) -> Self;
}

impl<const N: usize> PoseidonHasher<N> {
    pub fn new() -> Option<Self> {
        let light_params =
            light_poseidon::parameters::bn254_x5::get_poseidon_parameters((N + 1) as u8).ok()?;
        let parameters = light_poseidon_parameters_to_ark(light_params)?;

        Some(Self {
            parameters: Arc::new(parameters),
        })
    }

    /// Hashes `N` field elements, matching circomlib's `Poseidon(N)`.
    pub fn hash(&self, input: [Fr; N]) -> Fr {
        let mut state = Vec::with_capacity(N + 1);
        state.push(Fr::zero());
        state.extend_from_slice(&input);

        permute(&self.parameters, &mut state)
            .expect("poseidon permutation is infallible over native field elements");
        state[0]
    }
}

impl<const N: usize> PoseidonHasherGadget<N> {
    pub fn hash(&self, input: &[FrVar; N]) -> Result<FrVar, SynthesisError> {
        let mut state = Vec::with_capacity(N + 1);
        state.push(FrVar::zero());
        state.extend_from_slice(input);

        permute(&self.parameters, &mut state)?;
        Ok(state[0].clone())
    }
}

impl<const N: usize> AllocVar<PoseidonHasher<N>, Fr> for PoseidonHasherGadget<N> {
    fn new_variable<T: Borrow<PoseidonHasher<N>>>(
        _cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        debug_assert!(
            mode == AllocationMode::Constant,
            "PoseidonHasher Gadget is always allocated as a constant"
        );

        let value = f()?;
        Ok(Self {
            parameters: value.borrow().parameters.clone(),
        })
    }
}

impl PoseidonElem for Fr {
    fn add_constant(&mut self, constant: Fr) {
        *self += constant;
    }

    fn pow_alpha(&self, alpha: u64) -> Result<Self, SynthesisError> {
        Ok(self.pow([alpha]))
    }

    fn mul_constant(&self, constant: Fr) -> Self {
        *self * constant
    }
}

impl PoseidonElem for FrVar {
    fn add_constant(&mut self, constant: Fr) {
        *self += constant;
    }

    fn pow_alpha(&self, alpha: u64) -> Result<Self, SynthesisError> {
        self.pow_by_constant([alpha])
    }

    fn mul_constant(&self, constant: Fr) -> Self {
        self * constant
    }
}

/// Applies the Poseidon permutation to `state` in place, over either native
/// field elements or their in-circuit counterparts.
fn permute<E: PoseidonElem>(
    params: &PoseidonConfig<Fr>,
    state: &mut Vec<E>,
) -> Result<(), SynthesisError> {
    let half_full = params.full_rounds / 2;
    let partial_end = half_full + params.partial_rounds;

    for round in 0..(params.full_rounds + params.partial_rounds) {
        for (i, elem) in state.iter_mut().enumerate() {
            elem.add_constant(params.ark[round][i]);
        }

        if round < half_full || round >= partial_end {
            for elem in state.iter_mut() {
                *elem = elem.pow_alpha(params.alpha)?;
            }
        } else {
            state[0] = state[0].pow_alpha(params.alpha)?;
        }

        *state = (0..state.len())
            .map(|i| {
                state
                    .iter()
                    .enumerate()
                    .map(|(j, elem)| elem.mul_constant(params.mds[i][j]))
                    .reduce(|acc, term| acc + term)
                    .expect("poseidon state is never empty")
            })
            .collect();
    }

    Ok(())
}

/// Converts light-poseidon's PoseidonParameters to arkworks' PoseidonConfig.
fn light_poseidon_parameters_to_ark<F>(
    params: light_poseidon::PoseidonParameters<F>,
) -> Option<PoseidonConfig<F>>
where
    F: PrimeField,
{
    let width = params.width;

    if width < 2 {
        return None;
    }

    if params.ark.len() % width != 0 {
        return None;
    }

    let ark: Vec<Vec<F>> = params
        .ark
        .chunks(width)
        .map(|chunk| chunk.to_vec())
        .collect();

    let capacity = 1;
    let rate = width - capacity;

    Some(PoseidonConfig {
        full_rounds: params.full_rounds,
        partial_rounds: params.partial_rounds,
        alpha: params.alpha,
        ark,
        mds: params.mds,
        rate,
        capacity,
    })
}

#[cfg(test)]
mod tests {
    use ark_ff::UniformRand;
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;
    use ark_std::rand::Rng;
    use light_poseidon::PoseidonHasher;

    use super::*;

    /// Test that the PoseidonHasher matches the output of the light-poseidon crate.
    #[test]
    fn test_ark_matches_light_poseidon_2() {
        let mut rng = ark_std::test_rng();
        check_matches::<2>(10, &mut rng);
    }

    #[test]
    fn test_ark_matches_light_poseidon_5() {
        let mut rng = ark_std::test_rng();
        check_matches::<5>(10, &mut rng);
    }

    #[test]
    fn test_ark_matches_light_poseidon_8() {
        let mut rng = ark_std::test_rng();
        check_matches::<8>(10, &mut rng);
    }

    /// Tests a given PoseidonHasher<N> against light-poseidon.
    fn check_matches<const N: usize>(n: usize, rng: &mut impl Rng) {
        let mut light_hasher = light_poseidon::Poseidon::<Fr>::new_circom(N).unwrap();
        let ark_hasher = super::PoseidonHasher::<N>::new().unwrap();
        let cs = ConstraintSystem::new_ref();
        let ark_hasher_gadget = super::PoseidonHasherGadget::<N>::new_variable(
            cs.clone(),
            || Ok(&ark_hasher),
            AllocationMode::Constant,
        )
        .unwrap();

        for _ in 0..n {
            let inputs: [Fr; N] = core::array::from_fn(|_| Fr::rand(rng));
            let inputs_var: [FrVar; N] =
                core::array::from_fn(|i| FrVar::new_witness(cs.clone(), || Ok(inputs[i])).unwrap());

            let light_hash = light_hasher.hash(&inputs).unwrap();
            let ark_hash = ark_hasher.hash(inputs);
            let ark_hash_var = ark_hasher_gadget.hash(&inputs_var).unwrap();

            assert_eq!(
                light_hash, ark_hash,
                "PoseidonHasher output does not match light-poseidon",
            );

            assert_eq!(
                ark_hash_var.value().unwrap(),
                ark_hash,
                "PoseidonHasherGadget output does not match PoseidonHasher",
            );
        }

        assert!(
            cs.is_satisfied().unwrap(),
            "PoseidonHasherGadget constraints are not satisfied"
        );
    }
}

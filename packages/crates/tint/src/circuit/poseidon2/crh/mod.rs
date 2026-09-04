use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_crypto_primitives::crh::CRHScheme;

use crate::circuit::poseidon2::poseidon2_compress;

pub mod constraints;

/// Width of the underlying permutation.
const WIDTH: usize = 8;
/// Number of new elements absorbed per round.
const RATE: usize = WIDTH - 1;

/// Chained poseidon2 compression function, absorbing an arbitrary number of
/// field elements into a single field element.
#[derive(Clone, Debug, Default)]
pub struct Poseidon2ChainCrh;

impl CRHScheme for Poseidon2ChainCrh {
    type Input = [Fr];
    type Output = Fr;
    type Parameters = ();

    fn setup<R: ark_std::rand::Rng>(
        _r: &mut R,
    ) -> Result<Self::Parameters, ark_crypto_primitives::Error> {
        Ok(())
    }

    fn evaluate<T: Borrow<Self::Input>>(
        _parameters: &Self::Parameters,
        input: T,
    ) -> Result<Self::Output, ark_crypto_primitives::Error> {
        let mut state = Fr::from(0u64);
        for chunk in input.borrow().chunks(RATE) {
            let mut block = [Fr::from(0u64); WIDTH];
            block[0] = state;
            block[1..=chunk.len()].copy_from_slice(chunk);
            state = poseidon2_compress(&block);
        }
        Ok(state)
    }
}

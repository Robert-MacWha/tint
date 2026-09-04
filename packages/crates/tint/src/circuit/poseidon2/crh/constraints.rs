use ark_bn254::Fr;
use ark_crypto_primitives::crh::CRHSchemeGadget;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::gr1cs::SynthesisError;

use crate::circuit::{
    FrVar,
    poseidon2::{
        crh::{Poseidon2ChainCrh, RATE, WIDTH},
        poseidon2_compress_gadget,
    },
};

/// In-circuit counterpart of [`Poseidon2ChainCrh`].
#[derive(Clone, Debug, Default)]
pub struct Poseidon2ChainCrhGadget;

impl CRHSchemeGadget<Poseidon2ChainCrh, Fr> for Poseidon2ChainCrhGadget {
    type InputVar = [FrVar];
    type OutputVar = FrVar;
    type ParametersVar = ();

    fn evaluate(
        _parameters: &Self::ParametersVar,
        input: &Self::InputVar,
    ) -> Result<Self::OutputVar, SynthesisError> {
        let mut state = FrVar::zero();
        for chunk in input.chunks(RATE) {
            let mut block: [FrVar; WIDTH] = std::array::from_fn(|_| FrVar::zero());
            block[0] = state.clone();
            block[1..=chunk.len()].clone_from_slice(chunk);
            state = poseidon2_compress_gadget(&block)?;
        }
        Ok(state)
    }
}

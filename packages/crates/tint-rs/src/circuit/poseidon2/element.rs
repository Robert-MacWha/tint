use std::ops::Add;

use ark_bn254::Fr;
use ark_ff::Field;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::gr1cs::SynthesisError;

use crate::circuit::FrVar;

/// A native field element (`Fr`) or its in-circuit counterpart (`FrVar`).
pub trait PoseidonElement: Sized + Clone + Add<Self, Output = Self> {
    type Error;

    fn add_constant(&mut self, constant: Fr);
    fn pow_alpha(&self, alpha: u64) -> Result<Self, Self::Error>;
    fn mul_constant(&self, constant: Fr) -> Self;
}

impl PoseidonElement for Fr {
    type Error = ();

    fn add_constant(&mut self, constant: Fr) {
        *self += constant;
    }

    fn pow_alpha(&self, alpha: u64) -> Result<Self, Self::Error> {
        Ok(self.pow([alpha]))
    }

    fn mul_constant(&self, constant: Fr) -> Self {
        *self * constant
    }
}

impl PoseidonElement for FrVar {
    type Error = SynthesisError;

    fn add_constant(&mut self, constant: Fr) {
        *self += constant;
    }

    fn pow_alpha(&self, alpha: u64) -> Result<Self, Self::Error> {
        self.pow_by_constant([alpha])
    }

    fn mul_constant(&self, constant: Fr) -> Self {
        self * constant
    }
}

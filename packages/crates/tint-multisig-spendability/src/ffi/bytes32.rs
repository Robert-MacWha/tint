use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use num_bigint::BigUint;

use crate::ffi::bindings::TintBytes32;

impl From<Fr> for TintBytes32 {
    fn from(f: Fr) -> Self {
        let be = f.into_bigint().to_bytes_be();
        let mut data = [0u8; 32];
        data[32 - be.len()..].copy_from_slice(&be);
        TintBytes32 { data }
    }
}

impl From<TintBytes32> for Fr {
    fn from(b: TintBytes32) -> Self {
        Fr::from_be_bytes_mod_order(&b.data)
    }
}

impl From<&BigUint> for TintBytes32 {
    fn from(v: &BigUint) -> Self {
        let be = v.to_bytes_be();
        let mut data = [0u8; 32];
        data[32 - be.len()..].copy_from_slice(&be);
        TintBytes32 { data }
    }
}

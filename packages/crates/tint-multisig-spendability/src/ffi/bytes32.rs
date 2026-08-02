use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};

use crate::ffi::bindings::Bytes32;

impl From<Fr> for Bytes32 {
    fn from(f: Fr) -> Self {
        let be = f.into_bigint().to_bytes_be();
        let mut data = [0u8; 32];
        data[32 - be.len()..].copy_from_slice(&be);
        Bytes32 { data }
    }
}

impl From<Bytes32> for Fr {
    fn from(b: Bytes32) -> Self {
        Fr::from_be_bytes_mod_order(&b.data)
    }
}

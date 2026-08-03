#![cfg(test)]
use ark_bn254::Fr;
use tint::circuit::poseidon2::poseidon2_compress;

use crate::ffi::bindings::{
    Bytes32, Poseidon2Compress1, Poseidon2Compress2, Poseidon2Compress3, Poseidon2Compress8,
};

macro_rules! poseidon2_compress_fn {
    ($name:ident, $ffi:ident, $($arg:ident),+) => {
        pub fn $name($($arg: Fr),+) -> Fr {
            $(let mut $arg = Bytes32::from($arg);)+
            let mut out = Bytes32 { data: [0; 32] };
            unsafe { $ffi($(&mut $arg),+, &mut out) };
            out.into()
        }
    };
}

poseidon2_compress_fn!(poseidon2_compress1_via_go, Poseidon2Compress1, a);
poseidon2_compress_fn!(poseidon2_compress2_via_go, Poseidon2Compress2, a, b);
poseidon2_compress_fn!(poseidon2_compress3_via_go, Poseidon2Compress3, a, b, c);
poseidon2_compress_fn!(
    poseidon2_compress8_via_go,
    Poseidon2Compress8,
    a,
    b,
    c,
    d,
    e,
    f,
    g,
    h
);

#[test]
fn poseidon2_compress1_matches_go() {
    let a = Fr::from(1u64);
    assert_eq!(poseidon2_compress(&[a]), poseidon2_compress1_via_go(a));
}

#[test]
fn poseidon2_compress2_matches_go() {
    let (a, b) = (Fr::from(1u64), Fr::from(2u64));
    assert_eq!(
        poseidon2_compress(&[a, b]),
        poseidon2_compress2_via_go(a, b)
    );
}

#[test]
fn poseidon2_compress3_matches_go() {
    let (a, b, c) = (Fr::from(1u64), Fr::from(2u64), Fr::from(3u64));
    assert_eq!(
        poseidon2_compress(&[a, b, c]),
        poseidon2_compress3_via_go(a, b, c)
    );
}

#[test]
fn poseidon2_compress8_matches_go() {
    let input: [Fr; 8] = std::array::from_fn(|i| Fr::from((i + 1) as u64));
    assert_eq!(
        poseidon2_compress(&input),
        poseidon2_compress8_via_go(
            input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7]
        )
    );
}

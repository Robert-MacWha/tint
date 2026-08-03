#![cfg(test)]

use k256::{
    ecdsa::{SigningKey, VerifyingKey},
    elliptic_curve::Generate,
};

use crate::{
    N_SIGNERS,
    ffi::bindings::{Bytes32, PubKeyHash, TintPubKeyXY},
    pubkey_hash::pubkey_hash,
};

#[test]
fn pubkey_hash_matches_go() {
    let pub_keys: [VerifyingKey; N_SIGNERS] =
        std::array::from_fn(|_| *SigningKey::generate().verifying_key());

    let mut arr: [TintPubKeyXY; N_SIGNERS] =
        tint::array::try_from_fn(|i| TintPubKeyXY::try_from(&pub_keys[i])).unwrap();
    let mut out = Bytes32 { data: [0; 32] };
    unsafe { PubKeyHash(arr.as_mut_ptr(), &mut out) };

    assert_eq!(pubkey_hash(&pub_keys).unwrap(), out.into());
}

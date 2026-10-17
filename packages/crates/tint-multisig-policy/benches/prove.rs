#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use alloy_primitives::Address;
use ark_bn254::Fr;
use k256::{
    ecdsa::{Signature, SigningKey, VerifyingKey, signature::hazmat::PrehashSigner},
    elliptic_curve::Generate,
};
use tint::{
    circuit::join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
    fr::{address_to_fr, fr_to_b256},
    operation::OperationWitness,
};
use tint_multisig_policy::{N_SIGNERS, ffi, pubkey_hash};
use tint_poseidon2::poseidon2_compress;

fn main() {
    let contract_address = Address::new([9u8; 20]);

    let signers: [SigningKey; N_SIGNERS] = std::array::from_fn(|_| SigningKey::generate());
    let pub_keys: [VerifyingKey; N_SIGNERS] = std::array::from_fn(|i| *signers[i].verifying_key());
    let witness = pubkey_hash::pubkey_hash(&pub_keys).unwrap();

    let mut operation: OperationWitness<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> =
        OperationWitness::default();
    operation.inputs[0].note.policy.address = contract_address;
    operation.inputs[0].note.policy.witness = witness;

    let context_hash = Fr::from(0);
    let msg = fr_to_b256(poseidon2_compress(&[operation.hash(), context_hash])).0;
    let signatures: [Option<Signature>; N_SIGNERS] =
        tint::array::try_from_fn(|i| signers[i].sign_prehash(&msg).map(Some)).unwrap();

    let artifacts = ffi::artifacts::load_artifacts().unwrap();

    let prove_start = Instant::now();
    let _ = ffi::prove_via_go(
        &artifacts,
        address_to_fr(contract_address),
        &operation,
        context_hash,
        &pub_keys,
        &signatures,
    )
    .unwrap();
    let prove_time = prove_start.elapsed();

    println!("groth16 prove+verify+solidity (go ffi): {prove_time:?}");
}

use std::time::Instant;

use alloy_primitives::Address;
use k256::{
    ecdsa::{Signature, SigningKey, VerifyingKey, signature::hazmat::PrehashSigner},
    elliptic_curve::Generate,
};
use tint::{
    circuit::join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
    fr::{address_to_fr, fr_to_b256},
    operation::Operation,
};
use tint_multisig_spendability::{N_SIGNERS, ffi, pubkey_hash};

fn main() {
    let contract_address = Address::new([9u8; 20]);

    let signers: [SigningKey; N_SIGNERS] = std::array::from_fn(|_| SigningKey::generate());
    let pub_keys: [VerifyingKey; N_SIGNERS] = std::array::from_fn(|i| *signers[i].verifying_key());
    let witness = pubkey_hash::pubkey_hash(&pub_keys).unwrap();

    let mut operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> = Operation::default();
    operation.inputs[0].spendability_address = contract_address;
    operation.inputs[0].spendability_witness = witness;

    let msg = fr_to_b256(operation.hash()).0;
    let signatures: [Option<Signature>; N_SIGNERS] =
        tint::array::try_from_fn(|i| signers[i].sign_prehash(&msg).map(Some)).unwrap();

    let ccs = ffi::artifacts::ccs_bytes().unwrap();
    let pk = ffi::artifacts::proving_key_bytes().unwrap();
    let vk = ffi::artifacts::verifying_key_bytes().unwrap();

    let prove_start = Instant::now();
    let _ = ffi::prove_via_go(
        &ccs,
        &pk,
        &vk,
        address_to_fr(contract_address),
        &operation,
        &pub_keys,
        &signatures,
    )
    .unwrap();
    let prove_time = prove_start.elapsed();

    println!("groth16 prove+verify+solidity (go ffi): {:?}", prove_time);
}

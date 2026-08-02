#![cfg(test)]

use alloy_primitives::{Address, B256, Bytes};
use ark_bn254::Fr;
use tint::{
    account::keys::NullifierKey,
    circuit::join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
    note::commitment::SpendableCommitment,
    operation::Operation,
};

use crate::ffi::bindings::{Bytes32, OperationHash};

#[test]
fn operation_hash_matches_go() {
    let mut op: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> = Operation::default();
    op.inputs[0] = SpendableCommitment::new(
        Address::new([1; 20]).into(),
        100,
        NullifierKey::default(),
        Address::new([2; 20]),
        Fr::from(3u64),
        Bytes::default(),
        B256::new([5; 32]),
    );

    let mut c_op = (&op).into();
    let mut out = Bytes32 { data: [0; 32] };
    unsafe { OperationHash(&mut c_op, &mut out) };

    assert_eq!(op.hash(), out.into());
}

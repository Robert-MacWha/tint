#![cfg(test)]

use alloy_primitives::{Address, B256, Bytes};
use ark_bn254::Fr;
use tint::{
    account::keys::NullifierKey,
    circuit::join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
    note::{commitment::Commitment, owned::OwnedNote, withdrawal::Withdrawal},
    operation::Operation,
    policy::Policy,
};

use crate::ffi::bindings::{Bytes32, OperationHash};

#[test]
fn operation_hash_matches_go() {
    let mut op: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> = Operation::default();
    let policy = Policy::new(Address::new([2; 20]), Fr::from(3u64));
    let owned = OwnedNote {
        note: Commitment::new(
            Address::new([1; 20]).into(),
            100,
            policy.hash(),
            NullifierKey::default().pub_key(),
            B256::new([5; 32]),
        ),
        nullifier_key: NullifierKey::default(),
        policy,
    };
    op.inputs[0] = owned.spendable(Bytes::default());
    op.withdrawals[0] = Withdrawal::new(Address::new([1; 20]).into(), 100, Address::new([6; 20]));

    let mut c_op = (&op).into();
    let mut out = Bytes32 { data: [0; 32] };
    unsafe { OperationHash(&raw mut c_op, &raw mut out) };

    assert_eq!(op.hash(), out.into());
}

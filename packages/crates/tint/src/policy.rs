use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_ff::AdditiveGroup;
use tint_poseidon2::poseidon2_compress;

use crate::fr::address_to_fr;

/// A note's spending policy.
///
/// Notes commit to `poseidon2(address, witness)`. At spend time the circuit
/// checks the witness against that hash, and the contract calls
/// `address.requireAuthorized(op)` with a per-operation policy input.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Policy {
    pub address: Address,
    pub witness: Fr,
}

impl Policy {
    /// No policy. Policy checks are skipped by the contract for address zero.
    pub const NONE: Policy = Policy {
        address: Address::ZERO,
        witness: Fr::ZERO,
    };

    #[must_use]
    pub fn new(address: Address, witness: Fr) -> Self {
        Self { address, witness }
    }

    /// The policy hash committed by notes bound to this policy.
    #[must_use]
    pub fn hash(&self) -> Fr {
        poseidon2_compress(&[address_to_fr(self.address), self.witness])
    }
}

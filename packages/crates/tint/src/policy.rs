use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_ff::AdditiveGroup;

use crate::{circuit::poseidon2::poseidon2_compress, fr::address_to_fr};

/// A note's spending policy: the contract that enforces it, and the private
/// witness that configures it.
///
/// Notes commit to `poseidon2(address, witness)`. At spend time the circuit
/// checks the witness against that hash, and the contract calls
/// `address.requireSpendable(op)` with a per-operation policy input.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Policy {
    pub address: Address,
    pub witness: Fr,
}

impl Policy {
    /// No policy. The contract skips the spendability check for address zero.
    pub const NONE: Policy = Policy {
        address: Address::ZERO,
        witness: Fr::ZERO,
    };

    #[must_use]
    pub fn new(address: Address, witness: Fr) -> Self {
        Self { address, witness }
    }

    /// The hash committed into notes bound to this policy.
    #[must_use]
    pub fn hash(&self) -> Fr {
        poseidon2_compress(&[address_to_fr(self.address), self.witness])
    }
}

use std::convert::Infallible;

use alloy_primitives::{Address, Bytes};
use ark_bn254::Fr;
use ark_ff::AdditiveGroup;
use tint_poseidon2::poseidon2_compress;

use crate::{fr::address_to_fr, operation::UnauthorizedOperation};

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

/// Authorizes an operation's inputs under their spending policies.
pub trait Authorizer {
    type Error;

    /// Policy inputs shaped like [`Self::authorize`]'s, without authorizing.
    ///
    /// They only pass Tint while its simulation flag is set, but cost as much
    /// gas as real inputs, so an operation can be gas-estimated before it's
    /// authorized.
    fn placeholder(&self, op: &UnauthorizedOperation) -> Vec<Bytes>;

    /// One policy input per live input of `op`, in input order, as
    /// [`UnauthorizedOperation::authorize`] expects.
    ///
    /// # Errors
    ///
    /// Errors if the policy can't authorize `op`.
    fn authorize(&self, op: &UnauthorizedOperation) -> Result<Vec<Bytes>, Self::Error>;
}

/// Authorizes inputs under [`Policy::NONE`], which take empty policy inputs.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoPolicy;

impl Authorizer for NoPolicy {
    type Error = Infallible;

    fn placeholder(&self, op: &UnauthorizedOperation) -> Vec<Bytes> {
        vec![Bytes::new(); op.operation().live_inputs().count()]
    }

    fn authorize(&self, op: &UnauthorizedOperation) -> Result<Vec<Bytes>, Self::Error> {
        Ok(self.placeholder(op))
    }
}

use std::{array::from_fn, borrow::Borrow};

use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_hybrid_compression::circuit::{CompressedCircuit, CompressibleCircuit, Flatten};
use ark_r1cs_std::{
    GR1CSVar,
    alloc::{AllocVar, AllocationMode},
    eq::EqGadget,
};
use ark_relations::gr1cs::{ConstraintSystemRef, Namespace, SynthesisError};

use crate::{
    array::try_from_fn,
    circuit::{
        FrVar,
        operation::OperationVar,
        poseidon2::{
            crh::{Poseidon2ChainCrh, constraints::Poseidon2ChainCrhGadget},
            skew_mmr::Poseidon2Hasher,
        },
        variable, witness,
    },
    fr::{fr_to_address, fr_to_u128},
    note::asset::AssetId,
    operation::Operation,
};

pub const N_INPUTS: usize = 5;
pub const N_OUTPUTS: usize = 5;
pub const N_WITHDRAWALS: usize = 2;

/// Maximum depth of the MMR tree. Mirrors `Constants.sol`'s `MMR_MAX_DEPTH`.
pub const MMR_MAX_DEPTH: usize = 26;

/// Number of non-array public signals.  Mirrors `Constants.sol`'s `N_CONST`.
const N_CONST: usize = 3 + MMR_MAX_DEPTH;

/// Length of the flattened statement vector `JoinSplitResultVar`.  Mirrors
/// `Constants.sol`'s `N_PUB`.
pub const N_PUB: usize = N_CONST + 2 * N_INPUTS + N_OUTPUTS + 2 * N_WITHDRAWALS;

/// Concrete type alias for a compressed `JoinSplit` circuit, using `Poseidon2`
/// as its hash function.
pub type JoinSplitCircuit =
    CompressedCircuit<Fr, JoinSplit, Poseidon2ChainCrh, Poseidon2ChainCrhGadget, N_PUB>;

#[derive(Clone, Default)]
pub struct JoinSplit {
    pub hist_state: skew_mmr::state::State<MMR_MAX_DEPTH>,
    pub frontier: [Option<Fr>; MMR_MAX_DEPTH],
    pub bound_params_hash: Fr,
    pub inclusion_proofs: [skew_mmr::proof::Proof<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>; N_INPUTS],
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct JoinSplitVar {
    pub hist_state: skew_mmr::constraints::state::StateVar<MMR_MAX_DEPTH, Fr>,
    pub frontier: [FrVar; MMR_MAX_DEPTH],
    pub bound_params_hash: FrVar,
    pub inclusion_proofs:
        [skew_mmr::constraints::ProofVar<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>; N_INPUTS],
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct JoinSplitResult {
    // Circuit inputs.
    pub hist_state: skew_mmr::state::State<MMR_MAX_DEPTH>,
    pub frontier: [Fr; MMR_MAX_DEPTH],
    pub bound_params_hash: Fr,

    // Circuit outputs.
    pub operation_hash: Fr,
    pub nullifiers: [Fr; N_INPUTS],
    pub spendability_addresses: [Address; N_INPUTS],
    pub output_commitment_hashes: [Fr; N_OUTPUTS],
    pub withdrawal_amounts: [u128; N_WITHDRAWALS],
    pub withdrawal_assets: [AssetId; N_WITHDRAWALS],
}

pub struct JoinSplitResultVar {
    // Circuit inputs.
    pub hist_state: skew_mmr::constraints::state::StateVar<MMR_MAX_DEPTH, Fr>,
    pub frontier: [FrVar; MMR_MAX_DEPTH],
    pub bound_params_hash: FrVar,

    // Circuit outputs.
    pub operation_hash: FrVar,
    pub nullifiers: [FrVar; N_INPUTS],
    pub spendability_addresses: [FrVar; N_INPUTS],
    pub output_commitment_hashes: [FrVar; N_OUTPUTS],
    pub withdrawal_amounts: [FrVar; N_WITHDRAWALS],
    pub withdrawal_assets: [FrVar; N_WITHDRAWALS],
}

impl JoinSplit {
    pub fn new(
        hist_state: skew_mmr::state::State<MMR_MAX_DEPTH>,
        frontier: [Option<Fr>; MMR_MAX_DEPTH],
        bound_params_hash: Fr,
        inclusion_proofs: [skew_mmr::proof::Proof<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>; N_INPUTS],
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    ) -> Self {
        Self {
            hist_state,
            frontier,
            bound_params_hash,
            inclusion_proofs,
            operation,
        }
    }
}

impl CompressibleCircuit<Fr, N_PUB> for JoinSplit {
    type Output = JoinSplitResultVar;

    fn verify(&self, cs: &ConstraintSystemRef<Fr>) -> Result<Self::Output, SynthesisError> {
        let join_split_var: JoinSplitVar = witness(cs.clone(), self)?;
        join_split_var.verify()
    }
}

impl Flatten<Fr, N_PUB> for JoinSplitResultVar {
    /// Flattens this result into the ordered statement vector.
    fn flatten(&self) -> Result<[FrVar; N_PUB], SynthesisError> {
        let mut stmt = vec![
            self.bound_params_hash.clone(),
            self.operation_hash.clone(),
            self.hist_state.word().clone(),
        ];

        for i in 0..MMR_MAX_DEPTH {
            stmt.push(self.frontier[i].clone());
        }

        for i in 0..N_INPUTS {
            stmt.push(self.nullifiers[i].clone());
            stmt.push(self.spendability_addresses[i].clone());
        }

        for i in 0..N_OUTPUTS {
            stmt.push(self.output_commitment_hashes[i].clone());
        }

        for i in 0..N_WITHDRAWALS {
            stmt.push(self.withdrawal_amounts[i].clone());
            stmt.push(self.withdrawal_assets[i].clone());
        }

        stmt.try_into().map_err(|_| SynthesisError::ArityMismatch)
    }
}

impl JoinSplitVar {
    /// Verifies the `JoinSplit` operation.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify(&self) -> Result<JoinSplitResultVar, SynthesisError> {
        // TODO: Refactor me:
        // We should pass mmr_hist_state and mmr_frontier_roots as public inputs to the circuit. Then `proof.verify()`
        // should accept (hist_state, frontier_roots, element) as public inputs and verify them for its proof.
        // We should also produce the element as an output of operation.verify().  So code should look like:
        //
        // let input_commitment_hashes = self.operation.verify()?;
        // for (proof, element) in self.inclusion_proofs.iter().zip(input_commitment_hashes.iter()) {
        //   proof.verify(hist_state, frontier_roots, element)?;
        // }

        // Verify that the inclusion proofs are valid and all use the same MMR state.
        for proof in self.inclusion_proofs.iter() {
            //? Verify the proof's public states are all equal
            proof.state.enforce_equal(&self.hist_state)?;
            proof.roots.enforce_equal(&self.frontier)?;

            proof.verify()?;
        }

        // Verify that the operation is balanced and returns the resulting outputs.
        let input_commitment_hashes = from_fn(|i| self.inclusion_proofs[i].element.clone());
        let operation_result = self.operation.verify(&input_commitment_hashes)?;

        Ok(JoinSplitResultVar {
            hist_state: self.hist_state.clone(),
            frontier: self.frontier.clone(),
            bound_params_hash: self.bound_params_hash.clone(),
            operation_hash: operation_result.hash,
            nullifiers: operation_result.nullifiers,
            spendability_addresses: operation_result.spendability_addresses,
            output_commitment_hashes: operation_result.output_commitment_hashes,
            withdrawal_amounts: from_fn(|i| operation_result.withdrawals[i].amount.clone()),
            withdrawal_assets: from_fn(|i| operation_result.withdrawals[i].asset.clone()),
        })
    }
}

impl AllocVar<JoinSplit, Fr> for JoinSplitVar {
    fn new_variable<T: Borrow<JoinSplit>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let hist_state = variable(cs.clone(), &value.hist_state, mode)?;
        let frontier =
            try_from_fn(|i| variable(cs.clone(), &value.frontier[i].unwrap_or_default(), mode))?;
        let bound_params_hash = variable(cs.clone(), &value.bound_params_hash, mode)?;

        let inclusion_proofs = variable(cs.clone(), &value.inclusion_proofs, mode)?;
        let operation = variable(cs.clone(), &value.operation, mode)?;

        Ok(Self {
            hist_state,
            frontier,
            bound_params_hash,
            inclusion_proofs,
            operation,
        })
    }
}

impl TryFrom<JoinSplitResultVar> for JoinSplitResult {
    type Error = SynthesisError;

    fn try_from(value: JoinSplitResultVar) -> Result<Self, Self::Error> {
        let hist_state = value.hist_state.value()?;
        let frontier = value.frontier.value()?;
        let bound_params_hash = value.bound_params_hash.value()?;

        let operation_hash = value.operation_hash.value()?;
        let nullifiers = value.nullifiers.value()?;
        let spendability_addresses = value.spendability_addresses.value()?;
        let output_commitment_hashes = value.output_commitment_hashes.value()?;
        let withdrawal_amounts = value.withdrawal_amounts.value()?;
        let withdrawal_assets = value.withdrawal_assets.value()?;

        let spendability_addresses = from_fn(|i| fr_to_address(spendability_addresses[i]));
        let withdrawal_amounts = from_fn(|i| fr_to_u128(&withdrawal_amounts[i]));
        let withdrawal_assets = from_fn(|i| withdrawal_assets[i].into());

        Ok(Self {
            hist_state,
            frontier,
            bound_params_hash,
            operation_hash,
            nullifiers,
            spendability_addresses,
            output_commitment_hashes,
            withdrawal_amounts,
            withdrawal_assets,
        })
    }
}

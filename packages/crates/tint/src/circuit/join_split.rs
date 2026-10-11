use std::{array::from_fn, borrow::Borrow};

use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_hybrid_compression::circuit::{CompressedCircuit, CompressibleCircuit, Flatten};
use ark_r1cs_std::{
    GR1CSVar,
    alloc::{AllocVar, AllocationMode},
    boolean::Boolean,
    eq::EqGadget,
    fields::FieldVar,
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

/// Maximum depth of the MMR tree.
///
/// Mirrors `Constants.sol`'s `MMR_MAX_DEPTH`.
pub const MMR_MAX_DEPTH: usize = 42;

/// Number of non-array public signals.
///
/// Mirrors `Constants.sol`'s `N_CONST`.
const N_CONST: usize = 5;

/// Index of the first frontier root.
///
/// Mirrors `Constants.sol`'s `FRONTIER_OFFSET`.
pub const FRONTIER_OFFSET: usize = N_CONST + 2 * N_INPUTS + N_OUTPUTS + 2 * N_WITHDRAWALS;

/// Length of the flattened statement vector `JoinSplitResultVar`.
///
/// Mirrors `Constants.sol`'s `N_PUB`.
pub const N_PUB: usize = FRONTIER_OFFSET + MMR_MAX_DEPTH;

/// Concrete type alias for a compressed `JoinSplit` circuit, using `Poseidon2`
/// as its hash function.
pub type JoinSplitCircuit =
    CompressedCircuit<Fr, TintJoinSplit, Poseidon2ChainCrh, Poseidon2ChainCrhGadget, N_PUB>;

/// Concrete type alias for a `JoinSplit` circuit using tint's generics.
pub type TintJoinSplit = JoinSplit<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>;

#[derive(Clone)]
pub struct JoinSplit<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> {
    pub hist_state: skew_mmr::state::State<MMR_MAX_DEPTH>,
    pub frontier: [Option<Fr>; MMR_MAX_DEPTH],
    pub bound_params_hash: Fr,
    pub inclusion_proofs: [skew_mmr::proof::Proof<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>; N_INPUTS],
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct JoinSplitVar<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> {
    pub hist_state: skew_mmr::constraints::state::StateVar<MMR_MAX_DEPTH, Fr>,
    pub frontier: [FrVar; MMR_MAX_DEPTH],
    pub bound_params_hash: FrVar,
    pub inclusion_proofs:
        [skew_mmr::constraints::ProofVar<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>; N_INPUTS],
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinSplitResult<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> {
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

#[derive(Clone, Debug)]
pub struct JoinSplitResultVar<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> {
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

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> JoinSplit<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
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

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> CompressibleCircuit<Fr, N_PUB> for JoinSplit<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
    type Output = JoinSplitResultVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>;

    fn verify(&self, cs: &ConstraintSystemRef<Fr>) -> Result<Self::Output, SynthesisError> {
        let join_split_var: JoinSplitVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH> =
            witness(cs.clone(), self)?;
        join_split_var.verify()
    }
}

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> Flatten<Fr, N_PUB> for JoinSplitResultVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
    /// Flattens this result into the ordered statement vector.
    fn flatten(&self) -> Result<[FrVar; N_PUB], SynthesisError> {
        let mut stmt = vec![
            self.bound_params_hash.clone(),
            self.operation_hash.clone(),
            self.hist_state.packed_ranks().clone(),
            self.hist_state.depth().clone(),
            self.hist_state.count().clone(),
        ];

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

        for i in 0..MMR_MAX_DEPTH {
            stmt.push(self.frontier[i].clone());
        }

        stmt.try_into().map_err(|_| SynthesisError::ArityMismatch)
    }
}

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> JoinSplitVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
    /// Verifies the `JoinSplit` operation.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify(
        &self,
    ) -> Result<JoinSplitResultVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>, SynthesisError>
    {
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
        // Skipped for zero-valued elements (padding), which `OperationVar::verify` ties to
        // the unused (zero-amount) input slots.
        for proof in self.inclusion_proofs.iter() {
            let used = !proof.element.is_zero()?;

            //? Verify the proof's public states are all equal
            proof
                .state
                .conditional_enforce_equal(&self.hist_state, &used)?;
            proof
                .roots
                .conditional_enforce_equal(&self.frontier, &used)?;
            proof
                .verify()?
                .conditional_enforce_equal(&Boolean::TRUE, &used)?;
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

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> AllocVar<JoinSplit<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>, Fr>
    for JoinSplitVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
    fn new_variable<T: Borrow<JoinSplit<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>>>(
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

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> GR1CSVar<Fr> for JoinSplitResultVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
    type Value = JoinSplitResult<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>;

    fn cs(&self) -> ConstraintSystemRef<Fr> {
        self.hist_state.cs()
    }

    fn value(&self) -> Result<Self::Value, SynthesisError> {
        let hist_state = self.hist_state.value()?;
        let frontier = self.frontier.value()?;
        let bound_params_hash = self.bound_params_hash.value()?;

        let operation_hash = self.operation_hash.value()?;
        let nullifiers = self.nullifiers.value()?;
        let spendability_addresses = self.spendability_addresses.value()?;
        let output_commitment_hashes = self.output_commitment_hashes.value()?;
        let withdrawal_amounts = self.withdrawal_amounts.value()?;
        let withdrawal_assets = self.withdrawal_assets.value()?;

        let spendability_addresses = from_fn(|i| fr_to_address(spendability_addresses[i]));
        let withdrawal_amounts = from_fn(|i| fr_to_u128(&withdrawal_amounts[i]));
        let withdrawal_assets = from_fn(|i| withdrawal_assets[i].into());

        Ok(Self::Value {
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

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> Default for JoinSplit<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
    fn default() -> Self {
        Self {
            hist_state: Default::default(),
            frontier: from_fn(|_| None),
            bound_params_hash: Default::default(),
            inclusion_proofs: from_fn(|_| Default::default()),
            operation: Default::default(),
        }
    }
}

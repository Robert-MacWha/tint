use std::{array::from_fn, borrow::Borrow};

use ark_bn254::Fr;
use ark_ff::Fp;
use ark_hybrid_compression::circuit::{CompressedCircuit, CompressibleCircuit, Flatten};
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    boolean::Boolean,
    eq::EqGadget,
    fields::FieldVar,
};
use ark_relations::gr1cs::{ConstraintSystemRef, Namespace, SynthesisError};
use tint_poseidon2::{
    crh::{Poseidon2ChainCrh, constraints::Poseidon2ChainCrhGadget},
    skew_mmr::Poseidon2Hasher,
};

use crate::{
    array::try_from_fn,
    circuit::{FrVar, gadgets::operation::OperationVar, variable, witness},
    operation::Operation,
};

type SkewMmrProof<const MMR_MAX_DEPTH: usize> =
    skew_mmr::proof::Proof<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>;
type SkewMmrProofVar<const MMR_MAX_DEPTH: usize> =
    skew_mmr::constraints::ProofVar<MMR_MAX_DEPTH, Fr, Poseidon2Hasher>;
type SkewMmrState<const MMR_MAX_DEPTH: usize> = skew_mmr::state::State<MMR_MAX_DEPTH>;
type SkewMmrStateVar<const MMR_MAX_DEPTH: usize> =
    skew_mmr::constraints::state::StateVar<MMR_MAX_DEPTH, Fr>;

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
pub const FRONTIER_OFFSET: usize = frontier_offset(N_INPUTS, N_OUTPUTS, N_WITHDRAWALS);

/// Length of the flattened statement vector `JoinSplitResultVar`.
///
/// Mirrors `Constants.sol`'s `N_PUB`.
pub const N_PUB: usize = n_pub(N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH);

/// Index of the first frontier root for a `JoinSplit` of the given size.
#[must_use]
pub const fn frontier_offset(n_inputs: usize, n_outputs: usize, n_withdrawals: usize) -> usize {
    N_CONST + 2 * n_inputs + n_outputs + 3 * n_withdrawals
}

/// Length of the flattened statement vector for a `JoinSplit` of the given size.
#[must_use]
pub const fn n_pub(
    n_inputs: usize,
    n_outputs: usize,
    n_withdrawals: usize,
    mmr_max_depth: usize,
) -> usize {
    frontier_offset(n_inputs, n_outputs, n_withdrawals) + mmr_max_depth
}

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
    pub hist_state: SkewMmrState<MMR_MAX_DEPTH>,
    pub frontier: [Option<Fr>; MMR_MAX_DEPTH],
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub inclusion_proofs: [SkewMmrProof<MMR_MAX_DEPTH>; N_INPUTS],
    pub bound_params_hash: Fr,
}

pub struct JoinSplitVar<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> {
    pub hist_state: SkewMmrStateVar<MMR_MAX_DEPTH>,
    pub frontier: [FrVar; MMR_MAX_DEPTH],
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub inclusion_proofs: [SkewMmrProofVar<MMR_MAX_DEPTH>; N_INPUTS],
    pub bound_params_hash: FrVar,
}

#[derive(Clone, Debug)]
pub struct JoinSplitResultVar<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> {
    pub hist_state: SkewMmrStateVar<MMR_MAX_DEPTH>,
    pub frontier: [FrVar; MMR_MAX_DEPTH],
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    pub bound_params_hash: FrVar,
}

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
> JoinSplit<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
    pub fn new(
        hist_state: SkewMmrState<MMR_MAX_DEPTH>,
        frontier: [Option<Fr>; MMR_MAX_DEPTH],
        bound_params_hash: Fr,
        inclusion_proofs: [SkewMmrProof<MMR_MAX_DEPTH>; N_INPUTS],
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    ) -> Self {
        Self {
            hist_state,
            frontier,
            operation,
            inclusion_proofs,
            bound_params_hash,
        }
    }
}

impl<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
    const MMR_MAX_DEPTH: usize,
    const N_PUB: usize,
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
    const N_PUB: usize,
> Flatten<Fr, N_PUB> for JoinSplitResultVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH>
{
    /// Flattens this result into the ordered statement vector.
    fn flatten(&self) -> Result<[FrVar; N_PUB], SynthesisError> {
        const {
            assert!(
                N_PUB == n_pub(N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, MMR_MAX_DEPTH),
                "N_PUB does not match the JoinSplit's size",
            );
        }

        let mut stmt = vec![
            self.bound_params_hash.clone(),
            self.operation.hash()?,
            self.hist_state.packed_ranks().clone(),
            self.hist_state.depth().clone(),
            self.hist_state.count().clone(),
        ];

        let nullifiers = self.operation.nullifiers()?.into_iter();
        let policy_addresses = self.operation.policy_addresses().into_iter();
        for (nullifier, addr) in nullifiers.zip(policy_addresses) {
            stmt.push(nullifier);
            stmt.push(addr);
        }

        let output_commitment_hashes = self.operation.output_commitment_hashes()?.into_iter();
        for output_commitment in output_commitment_hashes {
            stmt.push(output_commitment);
        }

        for withdrawal in &self.operation.withdrawals {
            stmt.push(withdrawal.amount.to_fp()?);
            stmt.push(withdrawal.asset.clone());
            stmt.push(withdrawal.recipient.clone());
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
        self.operation.verify()?;

        let commitment_hashes = self.operation.input_commitment_hashes()?;
        for (proof, commitment_hash) in self.inclusion_proofs.iter().zip(commitment_hashes.iter()) {
            let used = !proof.element.is_zero()?;

            // Verify the proof's inputs match the expected values.
            proof
                .state
                .conditional_enforce_equal(&self.hist_state, &used)?;
            proof
                .roots
                .conditional_enforce_equal(&self.frontier, &used)?;
            proof
                .element
                .conditional_enforce_equal(commitment_hash, &used)?;

            // Verify the proof is valid.
            proof
                .verify()?
                .conditional_enforce_equal(&Boolean::TRUE, &used)?;
        }

        Ok(JoinSplitResultVar {
            hist_state: self.hist_state.clone(),
            frontier: self.frontier.clone(),
            operation: self.operation.clone(),
            bound_params_hash: self.bound_params_hash.clone(),
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
            operation,
            inclusion_proofs,
            bound_params_hash,
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
            hist_state: SkewMmrState::default(),
            frontier: from_fn(|_| None),
            bound_params_hash: Fp::default(),
            inclusion_proofs: from_fn(|_| SkewMmrProof::default()),
            operation: Operation::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Bytes, address};
    use ark_relations::gr1cs::ConstraintSystem;

    use super::*;
    use crate::note::{asset::AssetId, commitment::Commitment, owned::OwnedNote};

    const I: usize = 2;
    const O: usize = 2;
    const W: usize = 1;
    const D: usize = 8;
    const P: usize = n_pub(I, O, W, D);

    /// Expect that a small, balanced `JoinSplit` spending one note from the MMR
    /// is satisfied.
    #[test]
    fn small_join_split_satisfied() {
        let asset = AssetId::from(address!("0x00000000000000000000000000000000deadbeef"));

        let mut input = OwnedNote::default();
        input.note.asset = asset;
        input.note.amount = 10;
        input.note.policy_hash = input.policy.hash();

        let mut mmr = skew_mmr::SkewMmr::<D, Fr, Poseidon2Hasher>::new();
        mmr.append(Fr::from(1u64));
        mmr.append(input.commitment());
        mmr.append(Fr::from(2u64));
        let proof = mmr.prove_element(input.commitment()).unwrap();

        let mut operation = Operation::<I, O, W>::default();
        operation.inputs[0] = input.spendable(Bytes::new());
        operation.outputs[0] = Commitment {
            asset,
            amount: 10,
            ..Commitment::default()
        };

        let join_split = JoinSplit::new(
            proof.state,
            proof.roots,
            Fr::from(3u64),
            [proof, SkewMmrProof::default()],
            operation,
        );

        let cs = ConstraintSystem::<Fr>::new_ref();
        let result = CompressibleCircuit::<Fr, P>::verify(&join_split, &cs).unwrap();
        let _: [FrVar; P] = result.flatten().unwrap();
        assert!(cs.is_satisfied().unwrap());
    }
}

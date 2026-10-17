use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, fields::FieldVar};
use ark_relations::gr1cs::{Namespace, SynthesisError};
use tint_poseidon2::poseidon2_compress_gadget;

use crate::{
    array::try_from_fn,
    circuit::{
        FrVar,
        gadgets::{commitment::CommitmentVar, owned::OwnedNoteVar, withdrawal::WithdrawalVar},
        variable,
    },
    operation::OperationWitness,
};

#[derive(Clone, Debug)]
pub struct OperationVar<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize> {
    pub inputs: [OwnedNoteVar; N_INPUTS],
    pub outputs: [CommitmentVar; N_OUTPUTS],
    pub withdrawals: [WithdrawalVar; N_WITHDRAWALS],
}

impl<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize>
    OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>
{
    /// Verifies the operation is balanced and its precomputed `nullifiers` are unique.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify(&self, nullifiers: &[FrVar; N_INPUTS]) -> Result<(), SynthesisError> {
        self.verify_balance()?;
        verify_nullifier_uniqueness(nullifiers)?;
        Ok(())
    }

    /// Returns the operation hash, which commits to all inputs, outputs, and withdrawals.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn hash(&self) -> Result<FrVar, SynthesisError> {
        self.hash_from(
            &self.input_commitment_hashes()?,
            &self.output_commitment_hashes()?,
        )
    }

    /// Returns the operation hash from precomputed input and output commitment hashes.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn hash_from(
        &self,
        input_commitment_hashes: &[FrVar; N_INPUTS],
        output_commitment_hashes: &[FrVar; N_OUTPUTS],
    ) -> Result<FrVar, SynthesisError> {
        let mut hash = FrVar::zero();
        for i in input_commitment_hashes {
            hash = poseidon2_compress_gadget(&[hash.clone(), i.clone()])?;
        }

        for commitment_hash in output_commitment_hashes {
            hash = poseidon2_compress_gadget(&[hash.clone(), commitment_hash.clone()])?;
        }

        for withdrawal_hash in self.withdrawal_hashes()? {
            hash = poseidon2_compress_gadget(&[hash.clone(), withdrawal_hash.clone()])?;
        }

        Ok(hash)
    }

    /// Computes the commitment hash for each input.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn input_commitment_hashes(&self) -> Result<[FrVar; N_INPUTS], SynthesisError> {
        try_from_fn(|i| self.inputs[i].hash())
    }

    /// Computes the commitment hash for each output.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn output_commitment_hashes(&self) -> Result<[FrVar; N_OUTPUTS], SynthesisError> {
        try_from_fn(|i| self.outputs[i].hash())
    }

    /// Computes the nullifier for each input from its precomputed commitment hash.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn nullifiers(
        &self,
        input_commitment_hashes: &[FrVar; N_INPUTS],
    ) -> Result<[FrVar; N_INPUTS], SynthesisError> {
        try_from_fn(|i| self.inputs[i].nullifier(&input_commitment_hashes[i]))
    }

    /// Returns the policy address for each input
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn policy_addresses(&self) -> [FrVar; N_INPUTS] {
        std::array::from_fn(|i| self.inputs[i].policy_address.clone())
    }

    /// Computes the withdrawal hash for each withdrawal.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn withdrawal_hashes(&self) -> Result<[FrVar; N_WITHDRAWALS], SynthesisError> {
        try_from_fn(|i| self.withdrawals[i].hash())
    }

    /// Verifies that the operation is balanced.
    ///
    /// An operation is balanced when, for each asset, the sum of all inputs equals
    /// the sum of all outputs and withdrawals.
    ///
    /// Sums are computed in the field. Every amount is a `UInt128`, so a sum of
    /// `N_INPUTS + N_OUTPUTS + N_WITHDRAWALS` amounts cannot wrap the modulus.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn verify_balance(&self) -> Result<(), SynthesisError> {
        let inputs = self
            .inputs
            .iter()
            .map(|i| Ok((i.asset.clone(), i.amount.to_fp()?)))
            .collect::<Result<Vec<_>, SynthesisError>>()?;
        let commitments = self
            .outputs
            .iter()
            .map(|o| Ok((o.asset.clone(), o.amount.to_fp()?)));
        let withdrawals = self
            .withdrawals
            .iter()
            .map(|w| Ok((w.asset.clone(), w.amount.to_fp()?)));
        let outputs = commitments
            .chain(withdrawals)
            .collect::<Result<Vec<_>, SynthesisError>>()?;

        for (asset, _) in inputs.iter().chain(&outputs) {
            sum_for_asset(asset, &inputs)?.enforce_equal(&sum_for_asset(asset, &outputs)?)?;
        }

        Ok(())
    }
}

/// Calculates the sum of `amounts` whose asset matches `asset`.
#[tracing::instrument(target = "r1cs", skip_all)]
fn sum_for_asset(asset: &FrVar, amounts: &[(FrVar, FrVar)]) -> Result<FrVar, SynthesisError> {
    let mut sum = FrVar::zero();
    for (item_asset, amount) in amounts {
        let is_equal = asset.is_eq(item_asset)?;
        sum += is_equal.select(amount, &FrVar::zero())?;
    }
    Ok(sum)
}

/// Verifies that all unique nullifiers are unique, excluding sentinel nullifiers.
fn verify_nullifier_uniqueness<const N: usize>(
    nullifiers: &[FrVar; N],
) -> Result<(), SynthesisError> {
    for i in 0..N {
        let is_sentinel = !nullifiers[i].is_eq(&FrVar::zero())?;
        for j in (i + 1)..N {
            nullifiers[i].conditional_enforce_not_equal(&nullifiers[j], &is_sentinel)?;
        }
    }
    Ok(())
}

impl<const I: usize, const O: usize, const W: usize> AllocVar<OperationWitness<I, O, W>, Fr>
    for OperationVar<I, O, W>
{
    fn new_variable<T: Borrow<OperationWitness<I, O, W>>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let inputs = try_from_fn(|i| variable(cs.clone(), &value.inputs[i].note, mode))?;
        let outputs = variable(cs.clone(), &value.outputs, mode)?;
        let withdrawals = variable(cs.clone(), &value.withdrawals, mode)?;

        Ok(Self {
            inputs,
            outputs,
            withdrawals,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, address};
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;

    use super::*;
    use crate::circuit::witness;

    const DEAD_BEEF: Address = address!("0x00000000000000000000000000000000deadbeef");
    const C0FFEE: Address = address!("0x0000000000000000000000000000000000c0ffee");

    fn default_operation() -> OperationWitness<3, 3, 3> {
        let mut op = OperationWitness::<3, 3, 3>::default();
        op.inputs[0].note.note.asset = DEAD_BEEF.into();
        op.inputs[0].note.note.amount = 10;
        op.inputs[1].note.note.asset = DEAD_BEEF.into();
        op.inputs[1].note.note.amount = 10;
        op.inputs[2].note.note.asset = C0FFEE.into();
        op.inputs[2].note.note.amount = 10;
        op
    }

    /// Expect that an empty operation is valid.
    #[test]
    fn empty() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let op = OperationWitness::<3, 3, 3>::default();

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        let input_commitment_hashes = var.input_commitment_hashes().unwrap();
        var.verify(&var.nullifiers(&input_commitment_hashes).unwrap())
            .unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that a balanced operation is valid.
    #[test]
    fn balanced() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = default_operation();

        op.outputs[0].asset = DEAD_BEEF.into();
        op.outputs[0].amount = 15;
        op.outputs[1].asset = C0FFEE.into();
        op.outputs[1].amount = 10;
        op.withdrawals[0].asset = DEAD_BEEF.into();
        op.withdrawals[0].amount = 5;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        var.verify_balance().unwrap();
        assert!(cs.is_satisfied().unwrap());

        // Make the operation unbalanced and expect it to fail.
        op.outputs[0].amount = 10;
        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        var.verify_balance().unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }

    /// Expect that only amounts matching the asset are summed.
    #[test]
    fn sum_for_asset() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let asset_a: FrVar = witness(cs.clone(), &Fr::from(1)).unwrap();
        let asset_b: FrVar = witness(cs.clone(), &Fr::from(2)).unwrap();
        let asset_c: FrVar = witness(cs.clone(), &Fr::from(3)).unwrap();
        let amounts: [FrVar; 3] = [
            witness(cs.clone(), &Fr::from(10)).unwrap(),
            witness(cs.clone(), &Fr::from(15)).unwrap(),
            witness(cs.clone(), &Fr::from(20)).unwrap(),
        ];
        let items = [
            (asset_a.clone(), amounts[0].clone()),
            (asset_b.clone(), amounts[1].clone()),
            (asset_a.clone(), amounts[2].clone()),
        ];

        let sum = super::sum_for_asset(&asset_a, &items).unwrap();
        assert_eq!(sum.value().unwrap(), Fr::from(30));

        let sum = super::sum_for_asset(&asset_b, &items).unwrap();
        assert_eq!(sum.value().unwrap(), Fr::from(15));

        let sum = super::sum_for_asset(&asset_c, &items).unwrap();
        assert_eq!(sum.value().unwrap(), Fr::from(0));

        assert!(cs.is_satisfied().unwrap());
    }

    // Expect that nullifiers are unique, excluding sentinel nullifiers.
    #[test]
    fn nullifier_uniqueness() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let mut nullifiers = [
            witness(cs.clone(), &Fr::from(0)).unwrap(),
            witness(cs.clone(), &Fr::from(1)).unwrap(),
            witness(cs.clone(), &Fr::from(2)).unwrap(),
            witness(cs.clone(), &Fr::from(0)).unwrap(),
        ];
        verify_nullifier_uniqueness(&nullifiers).unwrap();
        assert!(cs.is_satisfied().unwrap());

        // Make the first two inputs have the same nullifier.
        nullifiers[0] = witness(cs.clone(), &Fr::from(1)).unwrap();
        verify_nullifier_uniqueness(&nullifiers).unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }
}

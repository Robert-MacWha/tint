use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, fields::FieldVar};
use ark_relations::gr1cs::{Namespace, SynthesisError};

use crate::{
    array::try_from_fn,
    circuit::{
        FrVar,
        commitment::{BaseCommitmentVar, SpendableCommitmentVar},
        variable,
    },
    note::withdrawal::Withdrawal,
    operation::Operation,
};

pub struct OperationVar<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize> {
    pub inputs: [SpendableCommitmentVar; N_INPUTS],
    pub output_commitments: [BaseCommitmentVar; N_OUTPUTS],
    pub withdrawals: [WithdrawalVar; N_WITHDRAWALS],
}

#[derive(Clone)]
pub struct WithdrawalVar {
    pub asset: FrVar,
    pub amount: FrVar,
}

pub struct OperationResult<
    const N_INPUTS: usize,
    const N_OUTPUTS: usize,
    const N_WITHDRAWALS: usize,
> {
    pub nullifiers: [FrVar; N_INPUTS],
    pub spendability_addresses: [FrVar; N_INPUTS],
    pub output_commitment_hashes: [FrVar; N_OUTPUTS],
    pub withdrawals: [WithdrawalVar; N_WITHDRAWALS],
}

impl<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize>
    OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>
{
    /// Verifies that the operation is balanced and returns the resulting outputs.
    ///
    /// A balanced operation means for each asset type the sum of inputs equals
    /// the sum of outputs.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify(
        &self,
        input_commitment_hashes: &[FrVar; N_INPUTS],
    ) -> Result<OperationResult<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>, SynthesisError> {
        self.verify_input_commitments(input_commitment_hashes)?;
        self.verify_spendability_hashes()?;
        self.enforce_u128()?;
        self.verify_balance()?;

        Ok(OperationResult {
            nullifiers: self.nullifiers(input_commitment_hashes)?,
            spendability_addresses: self.spendability_addresses(),
            output_commitment_hashes: self.output_commitment_hashes()?,
            withdrawals: self.withdrawals.clone(),
        })
    }

    /// Verifies that the inputs match the provided input commitment hashes,
    /// returning each input's computed base commitment hash for reuse.
    ///
    /// Skipped for unused (zero-amount) input slots.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn verify_input_commitments(
        &self,
        input_commitment_hashes: &[FrVar; N_INPUTS],
    ) -> Result<(), SynthesisError> {
        for i in 0..N_INPUTS {
            let computed_hash: FrVar = self.inputs[i].base.hash()?;
            let used = !self.inputs[i].base.amount.is_zero()?;
            let expected = used.select(&input_commitment_hashes[i], &computed_hash)?;
            computed_hash.enforce_equal(&expected)?;
        }
        Ok(())
    }

    /// Verifies that the inputs match the provided spendability hashes.
    ///
    /// Skipped for unused (zero-amount) input slots.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn verify_spendability_hashes(&self) -> Result<(), SynthesisError> {
        for i in 0..N_INPUTS {
            self.inputs[i].verify_spendability()?;
        }
        Ok(())
    }

    /// Computes the nullifier for each input, or `0` for unused (zero-amount) slots.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn nullifiers(
        &self,
        input_base_hashes: &[FrVar; N_INPUTS],
    ) -> Result<[FrVar; N_INPUTS], SynthesisError> {
        try_from_fn(|i| {
            let nullifier = self.inputs[i].nullifier(&input_base_hashes[i])?;
            let used = !self.inputs[i].base.amount.is_zero()?;
            used.select(&nullifier, &FrVar::zero())
        })
    }

    /// Returns the spendability address for each input
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn spendability_addresses(&self) -> [FrVar; N_INPUTS] {
        std::array::from_fn(|i| self.inputs[i].spendability_address.clone())
    }

    /// Computes the commitment hash for each output, or `0` for unused (zero-amount) slots.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn output_commitment_hashes(&self) -> Result<[FrVar; N_OUTPUTS], SynthesisError> {
        try_from_fn(|i| {
            let hash = self.output_commitments[i].hash()?;
            let used = !self.output_commitments[i].amount.is_zero()?;
            used.select(&hash, &FrVar::zero())
        })
    }

    /// Checks that all amounts in the operation fit in u128.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn enforce_u128(&self) -> Result<(), SynthesisError> {
        //? Don't need to enforce u128 for inputs, since they're enforced
        //? when notes are created.
        for output in &self.output_commitments {
            enforce_u128(&output.amount)?;
        }
        for output in &self.withdrawals {
            enforce_u128(&output.amount)?;
        }
        Ok(())
    }

    /// Verifies that the sum of inputs equals the sum of outputs for each asset type.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn verify_balance(&self) -> Result<(), SynthesisError> {
        let inputs = self.inputs.iter().map(|i| &i.base.asset);
        let commitments = self.output_commitments.iter().map(|o| &o.asset);
        let withdrawals = self.withdrawals.iter().map(|o| &o.asset);

        for asset in inputs.chain(commitments).chain(withdrawals) {
            self.input_sum_for_asset(asset)?
                .enforce_equal(&self.output_sum_for_asset(asset)?)?;
        }

        Ok(())
    }

    /// Calculates the sum of inputs for a given asset.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn input_sum_for_asset(&self, asset: &FrVar) -> Result<FrVar, SynthesisError> {
        let mut sum = FrVar::zero();
        for input in &self.inputs {
            let is_equal = asset.is_eq(&input.base.asset)?;
            let weighted = is_equal.select(&input.base.amount, &FrVar::zero())?;
            sum += &weighted;
        }
        Ok(sum)
    }

    /// Calculates the sum of outputs for a given asset.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn output_sum_for_asset(&self, asset: &FrVar) -> Result<FrVar, SynthesisError> {
        let mut sum = FrVar::zero();
        let commitments = self
            .output_commitments
            .iter()
            .map(|o| (&o.asset, &o.amount));
        let withdrawals = self.withdrawals.iter().map(|o| (&o.asset, &o.amount));

        let outputs = commitments.chain(withdrawals);
        for (out_asset, out_amount) in outputs {
            let is_equal = asset.is_eq(out_asset)?;
            sum += is_equal.select(out_amount, &FrVar::zero())?;
        }
        Ok(sum)
    }
}

impl WithdrawalVar {
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn enforce_equal(&self, other: &WithdrawalVar) -> Result<(), SynthesisError> {
        self.asset.enforce_equal(&other.asset)?;
        self.amount.enforce_equal(&other.amount)?;
        Ok(())
    }
}

impl<const I: usize, const O: usize, const W: usize> AllocVar<Operation<I, O, W>, Fr>
    for OperationVar<I, O, W>
{
    fn new_variable<T: Borrow<Operation<I, O, W>>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let inputs = try_from_fn(|i| variable(cs.clone(), &value.inputs[i], mode))?;
        let output_commitments =
            try_from_fn(|i| variable(cs.clone(), &value.output_commitments[i], mode))?;
        let output_withdrawals =
            try_from_fn(|i| variable(cs.clone(), &value.output_withdrawals[i], mode))?;

        Ok(Self {
            inputs,
            output_commitments,
            withdrawals: output_withdrawals,
        })
    }
}

impl AllocVar<Withdrawal, Fr> for WithdrawalVar {
    fn new_variable<T: Borrow<Withdrawal>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let asset = variable(cs.clone(), &value.asset_fr(), mode)?;
        let amount = variable(cs.clone(), &value.amount_fr(), mode)?;

        Ok(Self { asset, amount })
    }
}

/// Enforces that a field element fits in [0, 2^128).
#[tracing::instrument(target = "r1cs", skip_all)]
fn enforce_u128(v: &FrVar) -> Result<(), SynthesisError> {
    let _ = v.to_bits_le_with_top_bits_zero(128)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, address};
    use ark_r1cs_std::{GR1CSVar, fields::fp::FpVar};
    use ark_relations::gr1cs::ConstraintSystem;

    use crate::note::commitment::Commitment;

    use super::*;

    const DEAD_BEEF: Address = address!("0x00000000000000000000000000000000deadbeef");
    const C0FFEE: Address = address!("0x0000000000000000000000000000000000c0ffee");

    fn default_operation() -> Operation<3, 3, 3> {
        let mut op = Operation::<3, 3, 3>::default();
        op.inputs[0].base.asset = DEAD_BEEF.into();
        op.inputs[0].base.amount = 10;
        op.inputs[1].base.asset = DEAD_BEEF.into();
        op.inputs[1].base.amount = 10;
        op.inputs[2].base.asset = C0FFEE.into();
        op.inputs[2].base.amount = 10;
        op
    }

    /// Expect that an empty operation is valid.
    #[test]
    fn empty() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let op = Operation::<3, 3, 3>::default();

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        var.verify_balance().unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that a balanced operation is valid.
    #[test]
    fn balanced() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = default_operation();

        op.output_commitments[0].asset = DEAD_BEEF.into();
        op.output_commitments[0].amount = 15;
        op.output_commitments[1].asset = C0FFEE.into();
        op.output_commitments[1].amount = 10;
        op.output_withdrawals[0].asset = DEAD_BEEF.into();
        op.output_withdrawals[0].amount = 5;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        var.verify_balance().unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that an unbalanced operation is invalid.
    #[test]
    fn unbalanced_unsatisfied() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = default_operation();

        op.output_commitments[0].asset = DEAD_BEEF.into();
        op.output_commitments[0].amount = 15;
        op.output_commitments[1].asset = C0FFEE.into();
        op.output_commitments[1].amount = 10;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        var.verify_balance().unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }

    /// Expect that an operation with an overflowed output amount is invalid.
    #[test]
    fn enforce_u128() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let op = default_operation();

        let mut var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        let overflow_amount = FpVar::new_witness(cs.clone(), || Ok(-Fr::from(5u64))).unwrap();
        var.output_commitments[2].amount = overflow_amount;

        var.enforce_u128().unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }

    /// Expect that the input sum is calculated correctly.
    #[test]
    fn input_sum_for_asset() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = Operation::<3, 3, 3>::default();

        op.inputs[0].base.asset = DEAD_BEEF.into();
        op.inputs[0].base.amount = 10;
        op.inputs[1].base.asset = DEAD_BEEF.into();
        op.inputs[1].base.amount = 10;
        op.inputs[2].base.asset = C0FFEE.into();
        op.inputs[2].base.amount = 10;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();

        let sum = var.input_sum_for_asset(&var.inputs[0].base.asset).unwrap();
        let expected_sum = FpVar::new_witness(cs.clone(), || Ok(Fr::from(20))).unwrap();
        sum.enforce_equal(&expected_sum).unwrap();
        assert!(cs.is_satisfied().unwrap());

        let sum = var.input_sum_for_asset(&var.inputs[2].base.asset).unwrap();
        let expected_sum = FpVar::new_witness(cs.clone(), || Ok(Fr::from(10))).unwrap();
        sum.enforce_equal(&expected_sum).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that the output sum is calculated correctly.
    #[test]
    fn output_sum_for_asset() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = Operation::<3, 3, 3>::default();

        op.output_commitments[0].asset = DEAD_BEEF.into();
        op.output_commitments[0].amount = 15;
        op.output_commitments[1].asset = C0FFEE.into();
        op.output_commitments[1].amount = 10;

        op.output_withdrawals[0].asset = DEAD_BEEF.into();
        op.output_withdrawals[0].amount = 5;
        op.output_withdrawals[2].asset = C0FFEE.into();
        op.output_withdrawals[2].amount = 10;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        let sum = var
            .output_sum_for_asset(&var.output_commitments[0].asset)
            .unwrap();
        let expected_sum = FpVar::new_witness(cs.clone(), || Ok(Fr::from(20))).unwrap();
        sum.enforce_equal(&expected_sum).unwrap();
        assert!(cs.is_satisfied().unwrap());

        let sum = var
            .output_sum_for_asset(&var.output_commitments[1].asset)
            .unwrap();
        let expected_sum = FpVar::new_witness(cs.clone(), || Ok(Fr::from(20))).unwrap();
        sum.enforce_equal(&expected_sum).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that unused (zero-amount) input slots reveal a nullifier of 0,
    /// while a used slot reveals its real nullifier.
    #[test]
    fn nullifiers_zero_for_unused_inputs() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = Operation::<3, 3, 3>::default();
        op.inputs[0].base.asset = DEAD_BEEF.into();
        op.inputs[0].base.amount = 10;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        let input_base_hashes: [FrVar; 3] =
            std::array::from_fn(|i| var.inputs[i].base.hash().unwrap());
        let nullifiers = var.nullifiers(&input_base_hashes).unwrap();

        assert_eq!(nullifiers[0].value().unwrap(), op.inputs[0].nullifier());
        assert_eq!(nullifiers[1].value().unwrap(), Fr::from(0));
        assert_eq!(nullifiers[2].value().unwrap(), Fr::from(0));
        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that unused (zero-amount) output slots reveal a commitment hash
    /// of 0, while a used slot reveals its real hash.
    #[test]
    fn output_commitment_hashes_zero_for_unused_outputs() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = Operation::<3, 3, 3>::default();
        op.output_commitments[0].asset = DEAD_BEEF.into();
        op.output_commitments[0].amount = 10;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        let hashes = var.output_commitment_hashes().unwrap();

        assert_eq!(hashes[0].value().unwrap(), op.output_commitments[0].hash());
        assert_eq!(hashes[1].value().unwrap(), Fr::from(0));
        assert_eq!(hashes[2].value().unwrap(), Fr::from(0));
        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that unused (zero-amount) input slots don't need their provided
    /// leaf to match the computed commitment hash.
    #[test]
    fn verify_input_commitments_ignores_unused_slots() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let op = Operation::<3, 3, 3>::default();

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        let mismatched_leaves: [FrVar; 3] =
            std::array::from_fn(|_| FpVar::new_witness(cs.clone(), || Ok(Fr::from(1234))).unwrap());

        let _ = var.verify_input_commitments(&mismatched_leaves).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Expect that used input slots still require their provided leaf to
    /// match the computed commitment hash.
    #[test]
    fn verify_input_commitments_enforces_used_slots() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = Operation::<3, 3, 3>::default();
        op.inputs[0].base.asset = DEAD_BEEF.into();
        op.inputs[0].base.amount = 10;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        let wrong_leaves: [FrVar; 3] =
            std::array::from_fn(|_| FpVar::new_witness(cs.clone(), || Ok(Fr::from(1234))).unwrap());

        let _ = var.verify_input_commitments(&wrong_leaves).unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }
}

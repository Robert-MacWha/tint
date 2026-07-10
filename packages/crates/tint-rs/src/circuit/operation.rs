use std::borrow::Borrow;

use ark_bn254::Fr;
use ark_r1cs_std::{
    alloc::AllocVar,
    boolean::Boolean,
    eq::EqGadget,
    fields::{FieldVar, fp::FpVar},
    prelude::ToBitsGadget,
};
use ark_relations::gr1cs::{Namespace, SynthesisError};

use crate::{
    circuit::{FrVar, try_array_from_fn, variable},
    circuits::inputs::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
    note::{commitment::Commitment, withdrawal::Withdrawal},
    operation::Operation,
};

pub struct OperationVar {
    pub inputs: [CommitmentVar; N_INPUTS],
    pub output_commitments: [CommitmentVar; N_OUTPUTS],
    pub output_withdrawals: [WithdrawalVar; N_WITHDRAWALS],
}

pub struct CommitmentVar {
    pub asset: FrVar,
    pub amount: FrVar,
    pub partial_commitment: PartialCommitmentVar,
}

pub struct WithdrawalVar {
    pub asset: FrVar,
    pub amount: FrVar,
}

pub struct PartialCommitmentVar {
    pub spendability_hash: FrVar,
    pub nullifier_pub_key: FrVar,
    pub random: FrVar,
}

impl OperationVar {
    /// Validates that there is no asset creation or destruction in the operation.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn validate(&self) -> Result<(), SynthesisError> {
        self.enforce_u128()?;
        self.check_balance()
    }

    /// Checks that all amounts in the operation fit in u128.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn enforce_u128(&self) -> Result<(), SynthesisError> {
        for input in &self.inputs {
            enforce_u128(&input.amount)?;
        }
        for output in &self.output_commitments {
            enforce_u128(&output.amount)?;
        }
        for output in &self.output_withdrawals {
            enforce_u128(&output.amount)?;
        }
        Ok(())
    }

    /// Checks that the sum of inputs equals the sum of outputs for each asset type.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn check_balance(&self) -> Result<(), SynthesisError> {
        let inputs = self.inputs.iter().map(|i| &i.asset);
        let commitments = self.output_commitments.iter().map(|o| &o.asset);
        let withdrawals = self.output_withdrawals.iter().map(|o| &o.asset);

        for asset in inputs.chain(commitments).chain(withdrawals) {
            self.input_sum_for_asset(asset)?
                .enforce_equal(&self.output_sum_for_asset(asset)?)?;
        }

        Ok(())
    }

    /// Calculates the sum of inputs for a given asset type.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn input_sum_for_asset(&self, asset: &FrVar) -> Result<FrVar, SynthesisError> {
        let mut sum = FrVar::zero();
        for input in &self.inputs {
            let is_equal = asset.is_eq(&input.asset)?;
            let weighted = is_equal.select(&input.amount, &FrVar::zero())?;
            sum += &weighted;
        }
        Ok(sum)
    }

    /// Calculates the sum of outputs for a given asset type.
    #[tracing::instrument(target = "r1cs", skip_all)]
    fn output_sum_for_asset(&self, asset: &FrVar) -> Result<FrVar, SynthesisError> {
        let mut sum = FrVar::zero();
        let commitments = self
            .output_commitments
            .iter()
            .map(|o| (&o.asset, &o.amount));
        let withdrawals = self
            .output_withdrawals
            .iter()
            .map(|o| (&o.asset, &o.amount));

        let outputs = commitments.chain(withdrawals);
        for (out_asset, out_amount) in outputs {
            let is_equal = asset.is_eq(out_asset)?;
            sum += is_equal.select(out_amount, &FrVar::zero())?;
        }
        Ok(sum)
    }
}

impl AllocVar<Operation, Fr> for OperationVar {
    fn new_variable<T: Borrow<Operation>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let inputs = try_array_from_fn(|i| {
            CommitmentVar::new_variable(cs.clone(), || Ok(&value.inputs[i]), mode)
        })?;
        let output_commitments = try_array_from_fn(|i| {
            CommitmentVar::new_variable(cs.clone(), || Ok(&value.output_commitments[i]), mode)
        })?;
        let output_withdrawals = try_array_from_fn(|i| {
            WithdrawalVar::new_variable(cs.clone(), || Ok(&value.output_withdrawals[i]), mode)
        })?;

        Ok(Self {
            inputs,
            output_commitments,
            output_withdrawals,
        })
    }
}

impl AllocVar<Commitment, Fr> for CommitmentVar {
    fn new_variable<T: Borrow<Commitment>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let asset = variable(cs.clone(), value.asset_fr(), mode)?;
        let amount = variable(cs.clone(), value.amount_fr(), mode)?;
        let spendability_hash = variable(cs.clone(), value.spendability_hash(), mode)?;
        let nullifier_pub_key = variable(cs.clone(), value.nullifying_pub_key(), mode)?;
        let random = variable(cs.clone(), value.random, mode)?;

        Ok(Self {
            asset,
            amount,
            partial_commitment: PartialCommitmentVar {
                spendability_hash,
                nullifier_pub_key,
                random,
            },
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

        let asset = variable(cs.clone(), value.asset_fr(), mode)?;
        let amount = variable(cs.clone(), value.amount_fr(), mode)?;

        Ok(Self { asset, amount })
    }
}

/// Enforces that a field element fits in [0, 2^128).
#[tracing::instrument(target = "r1cs", skip_all)]
fn enforce_u128(v: &FrVar) -> Result<(), SynthesisError> {
    let bits = v.to_bits_le()?;
    for bit in &bits[128..] {
        bit.enforce_equal(&Boolean::constant(false))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, address};
    use ark_relations::gr1cs::ConstraintSystem;

    use super::*;

    const DEAD_BEEF: Address = address!("0x00000000000000000000000000000000deadbeef");
    const C0FFEE: Address = address!("0x0000000000000000000000000000000000c0ffee");

    fn default_operation() -> Operation {
        let mut op = Operation::default();
        op.inputs[0].asset = DEAD_BEEF.into();
        op.inputs[0].amount = 10;
        op.inputs[1].asset = DEAD_BEEF.into();
        op.inputs[1].amount = 10;
        op.inputs[2].asset = C0FFEE.into();
        op.inputs[2].amount = 10;
        op
    }

    /// Tests that an empty operation is valid.
    #[test]
    fn test_empty() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let op = Operation::default();

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        var.validate().unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Tests that a balanced operation is valid.
    #[test]
    fn test_balanced() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = default_operation();

        op.output_commitments[0].asset = DEAD_BEEF.into();
        op.output_commitments[0].amount = 15;
        op.output_commitments[1].asset = C0FFEE.into();
        op.output_commitments[1].amount = 10;
        op.output_withdrawals[0].asset = DEAD_BEEF.into();
        op.output_withdrawals[0].amount = 5;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        var.validate().unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Tests that an unbalanced operation is invalid.
    #[test]
    fn test_unbalanced() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = default_operation();

        op.output_commitments[0].asset = DEAD_BEEF.into();
        op.output_commitments[0].amount = 15;
        op.output_commitments[1].asset = C0FFEE.into();
        op.output_commitments[1].amount = 10;

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        var.validate().unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }

    /// Tests that an operation with an overflowed output amount is invalid.
    #[test]
    fn test_enforce_u128() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let op = default_operation();

        let mut var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();
        let overflow_amount = FpVar::new_witness(cs.clone(), || Ok(-Fr::from(5u64))).unwrap();
        var.output_commitments[2].amount = overflow_amount;

        var.enforce_u128().unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }

    /// Tests that the input sum is calculated correctly.
    #[test]
    fn test_input_sum_for_asset() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let op = default_operation();

        let var = OperationVar::new_witness(cs.clone(), || Ok(&op)).unwrap();

        let sum = var.input_sum_for_asset(&var.inputs[0].asset).unwrap();
        let expected_sum = FpVar::new_witness(cs.clone(), || Ok(Fr::from(20))).unwrap();
        sum.enforce_equal(&expected_sum).unwrap();
        assert!(cs.is_satisfied().unwrap());

        let sum = var.input_sum_for_asset(&var.inputs[2].asset).unwrap();
        let expected_sum = FpVar::new_witness(cs.clone(), || Ok(Fr::from(10))).unwrap();
        sum.enforce_equal(&expected_sum).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    /// Tests that the output sum is calculated correctly.
    #[test]
    fn test_output_sum_for_asset() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut op = default_operation();

        op.output_commitments[0].asset = DEAD_BEEF.into();
        op.output_commitments[0].amount = 15;
        op.output_commitments[1].asset = C0FFEE.into();
        op.output_commitments[1].amount = 10;
        op.output_withdrawals[2].asset = DEAD_BEEF.into();
        op.output_withdrawals[2].amount = 5;
        op.output_withdrawals[3].asset = C0FFEE.into();
        op.output_withdrawals[3].amount = 10;

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
}

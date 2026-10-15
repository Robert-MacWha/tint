use ark_bn254::Fr;
use ark_r1cs_std::{alloc::AllocVar, uint128::UInt128};
use ark_relations::gr1cs::SynthesisError;
use tint_poseidon2::poseidon2_compress_gadget;

use crate::{
    circuit::{FrVar, variable},
    note::withdrawal::Withdrawal,
};

/// Circuit variable for a [`Withdrawal`].
#[derive(Clone, Debug)]
pub struct WithdrawalVar {
    pub asset: FrVar,
    pub amount: UInt128<Fr>,
    pub recipient: FrVar,
}

impl WithdrawalVar {
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn hash(&self) -> Result<FrVar, SynthesisError> {
        poseidon2_compress_gadget(&[
            self.asset.clone(),
            self.amount.to_fp()?,
            self.recipient.clone(),
        ])
    }
}

impl AllocVar<Withdrawal, Fr> for WithdrawalVar {
    fn new_variable<T: std::borrow::Borrow<Withdrawal>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let asset = variable(cs.clone(), &value.asset_fr(), mode)?;
        let amount = variable(cs.clone(), &value.amount, mode)?;
        let recipient = variable(cs.clone(), &value.recipient_fr(), mode)?;

        Ok(Self {
            asset,
            amount,
            recipient,
        })
    }
}

#[cfg(test)]
mod tests {
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;

    use super::*;
    use crate::circuit::witness;

    #[test]
    fn hash() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let withdrawal = Withdrawal::default();
        let withdrawal_var: WithdrawalVar = witness(cs.clone(), &withdrawal).unwrap();

        let withdrawal_hash = withdrawal.hash();
        let withdrawal_var_hash = withdrawal_var.hash().unwrap().value().unwrap();

        assert_eq!(withdrawal_hash, withdrawal_var_hash);
    }
}

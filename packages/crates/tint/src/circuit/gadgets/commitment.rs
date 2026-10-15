use ark_bn254::Fr;
use ark_r1cs_std::{
    alloc::AllocVar, eq::EqGadget, fields::FieldVar, prelude::Boolean, uint128::UInt128,
};
use ark_relations::gr1cs::SynthesisError;
use tint_poseidon2::poseidon2_compress_gadget;

use crate::{
    circuit::{FrVar, variable},
    note::commitment::Commitment,
};

/// Circuit variable for a [`Commitment`].
#[derive(Clone, Debug)]
pub struct CommitmentVar {
    pub asset: FrVar,
    pub amount: UInt128<Fr>,
    pub policy_hash: FrVar,
    pub nullifying_pub_key: FrVar,
    pub random: FrVar,
}

impl CommitmentVar {
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn hash(&self) -> Result<FrVar, SynthesisError> {
        self.is_sentinel()?.select(
            &FrVar::zero(),
            &poseidon2_compress_gadget(&[
                self.asset.clone(),
                self.amount.to_fp()?,
                self.partial_hash()?,
            ])?,
        )
    }

    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn is_sentinel(&self) -> Result<Boolean<Fr>, SynthesisError> {
        self.amount.is_eq(&UInt128::constant(0u128))
    }

    #[tracing::instrument(target = "r1cs", skip_all)]
    fn partial_hash(&self) -> Result<FrVar, SynthesisError> {
        poseidon2_compress_gadget(&[
            self.policy_hash.clone(),
            self.nullifying_pub_key.clone(),
            self.random.clone(),
        ])
    }
}

impl AllocVar<Commitment, Fr> for CommitmentVar {
    fn new_variable<T: std::borrow::Borrow<Commitment>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let asset = variable(cs.clone(), &value.asset_fr(), mode)?;
        let amount = variable(cs.clone(), &value.amount, mode)?;
        let policy_hash = variable(cs.clone(), &value.policy_hash, mode)?;
        let nullifying_pub_key = variable(cs.clone(), &value.nullifier_pub_key.0, mode)?;
        let random = variable(cs.clone(), &value.random_fr(), mode)?;

        Ok(Self {
            asset,
            amount,
            policy_hash,
            nullifying_pub_key,
            random,
        })
    }
}

#[cfg(test)]
mod tests {
    use ark_ff::AdditiveGroup;
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;

    use super::*;
    use crate::circuit::witness;

    #[test]
    fn hash() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let commitment = Commitment {
            amount: 1234,
            ..Commitment::default()
        };

        let commitment_var: CommitmentVar = witness(cs.clone(), &commitment).unwrap();

        let commitment_hash = commitment.hash();
        let commitment_hash_var = commitment_var.hash().unwrap().value().unwrap();

        assert_eq!(commitment_hash, commitment_hash_var);
        assert_ne!(commitment_hash, Fr::ZERO);
    }

    #[test]
    fn hash_sentinel() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let commitment = Commitment::default();
        let commitment_var: CommitmentVar = witness(cs.clone(), &commitment).unwrap();

        let commitment_hash = commitment.hash();
        let commitment_hash_var = commitment_var.hash().unwrap().value().unwrap();

        assert_eq!(commitment_hash, commitment_hash_var);
        assert_eq!(commitment_hash, Fr::ZERO);
    }

    #[test]
    fn partial_hash() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let commitment = Commitment::default();
        let commitment_var: CommitmentVar = witness(cs.clone(), &commitment).unwrap();

        let partial_commitment_hash = commitment.partial_hash();
        let partial_commitment_hash_var = commitment_var.partial_hash().unwrap().value().unwrap();

        assert_eq!(partial_commitment_hash, partial_commitment_hash_var);
    }
}

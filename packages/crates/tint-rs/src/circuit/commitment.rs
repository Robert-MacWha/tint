use ark_bn254::Fr;
use ark_r1cs_std::alloc::AllocVar;
use ark_relations::gr1cs::SynthesisError;

use crate::{
    circuit::{FrVar, poseidon::poseidon_hash_gadget, variable},
    note::commitment::{BaseCommitment, Commitment, SpendableCommitment},
};

pub struct BaseCommitmentVar {
    pub asset: FrVar,
    pub amount: FrVar,
    pub spendability_hash: FrVar,
    pub nullifying_pub_key: FrVar,
    pub random: FrVar,
}

pub struct SpendableCommitmentVar {
    pub base: BaseCommitmentVar,
    pub nullifier: FrVar,
}

impl BaseCommitmentVar {
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn hash(&self) -> Result<FrVar, SynthesisError> {
        poseidon_hash_gadget(&[
            self.asset.clone(),
            self.amount.clone(),
            self.partial_hash()?,
        ])
    }

    #[tracing::instrument(target = "r1cs", skip_all)]
    fn partial_hash(&self) -> Result<FrVar, SynthesisError> {
        poseidon_hash_gadget(&[
            self.spendability_hash.clone(),
            self.nullifying_pub_key.clone(),
            self.random.clone(),
        ])
    }
}

impl SpendableCommitmentVar {
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn nullifier(&self) -> Result<FrVar, SynthesisError> {
        poseidon_hash_gadget(&[self.nullifier.clone(), self.base.hash()?])
    }
}

impl AllocVar<BaseCommitment, Fr> for BaseCommitmentVar {
    fn new_variable<T: std::borrow::Borrow<BaseCommitment>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let asset = variable(cs.clone(), &value.asset_fr(), mode)?;
        let amount = variable(cs.clone(), &value.amount_fr(), mode)?;
        let spendability_hash = variable(cs.clone(), &value.spendability_hash(), mode)?;
        let nullifying_pub_key = variable(cs.clone(), &value.nullifier_pub_key().0, mode)?;
        let random = variable(cs.clone(), &value.random_fr(), mode)?;

        Ok(Self {
            asset,
            amount,
            spendability_hash,
            nullifying_pub_key,
            random,
        })
    }
}

impl AllocVar<SpendableCommitment, Fr> for SpendableCommitmentVar {
    fn new_variable<T: std::borrow::Borrow<SpendableCommitment>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let base = BaseCommitmentVar::new_variable(cs.clone(), || Ok(&value.base), mode)?;
        let nullifier = variable(cs.clone(), &value.nullifier_key.0, mode)?;

        Ok(Self { base, nullifier })
    }
}

#[cfg(test)]
mod tests {
    use ark_r1cs_std::GR1CSVar;
    use ark_relations::gr1cs::ConstraintSystem;

    use crate::{
        circuit::witness,
        note::commitment::{BaseCommitment, Commitment, SpendableCommitment},
    };

    use super::*;

    #[test]
    fn commitment_hash() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let commitment = BaseCommitment::default();
        let commitment_var: BaseCommitmentVar = witness(cs.clone(), &commitment).unwrap();

        let commitment_hash = commitment.hash();
        let commitment_hash_var = commitment_var.hash().unwrap().value().unwrap();

        assert_eq!(commitment_hash, commitment_hash_var);
    }

    #[test]
    fn partial_hash() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let commitment = BaseCommitment::default();
        let commitment_var: BaseCommitmentVar = witness(cs.clone(), &commitment).unwrap();

        let partial_commitment_hash = commitment.partial_hash();
        let partial_commitment_hash_var = commitment_var.partial_hash().unwrap().value().unwrap();

        assert_eq!(partial_commitment_hash, partial_commitment_hash_var);
    }

    #[test]
    fn test_nullifier() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let commitment = SpendableCommitment::default();
        let commitment_var: SpendableCommitmentVar = witness(cs.clone(), &commitment).unwrap();

        let nullifier = commitment.nullifier();
        let nullifier_var = commitment_var.nullifier().unwrap();

        assert_eq!(nullifier, nullifier_var.value().unwrap());
    }
}

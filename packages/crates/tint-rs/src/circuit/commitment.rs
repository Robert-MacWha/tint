use ark_bn254::Fr;
use ark_r1cs_std::alloc::AllocVar;
use ark_relations::gr1cs::SynthesisError;

use crate::{
    circuit::{FrVar, poseidon::poseidon_hash_gadget, variable},
    note::commitment::{BaseCommitment, KeyMaterial, NullifierKey, NullifierPubKey},
};

pub trait KeyMaterialVar {
    type Native: KeyMaterial;
    fn nullifying_pub_key(&self) -> Result<FrVar, SynthesisError>;
}

pub struct BaseCommitmentVar<K: KeyMaterialVar> {
    pub asset: FrVar,
    pub amount: FrVar,
    pub spendability_hash: FrVar,
    pub key: K,
    pub random: FrVar,
}

pub struct NullifierKeyVar(pub FrVar);
pub struct NullifierPubKeyVar(pub FrVar);

pub type CommitmentVar = BaseCommitmentVar<NullifierKeyVar>;
pub type SpendableCommitmentVar = BaseCommitmentVar<NullifierPubKeyVar>;

impl<K: KeyMaterialVar> BaseCommitmentVar<K> {
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn commitment_hash(&self) -> Result<FrVar, SynthesisError> {
        poseidon_hash_gadget(&[
            self.asset.clone(),
            self.amount.clone(),
            self.partial_commitment_hash()?,
        ])
    }

    #[tracing::instrument(target = "r1cs", skip_all)]
    fn partial_commitment_hash(&self) -> Result<FrVar, SynthesisError> {
        poseidon_hash_gadget(&[
            self.spendability_hash.clone(),
            self.key.nullifying_pub_key()?,
            self.random.clone(),
        ])
    }
}

impl BaseCommitmentVar<NullifierKeyVar> {
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn nullifier(&self) -> Result<FrVar, SynthesisError> {
        poseidon_hash_gadget(&[self.key.0.clone()])
    }
}

impl KeyMaterialVar for NullifierKeyVar {
    type Native = NullifierKey;

    fn nullifying_pub_key(&self) -> Result<FrVar, SynthesisError> {
        poseidon_hash_gadget(&[self.0.clone()])
    }
}

impl KeyMaterialVar for NullifierPubKeyVar {
    type Native = NullifierPubKey;

    fn nullifying_pub_key(&self) -> Result<FrVar, SynthesisError> {
        Ok(self.0.clone())
    }
}

impl<K: KeyMaterialVar> AllocVar<BaseCommitment<K::Native>, Fr> for BaseCommitmentVar<K>
where
    K: AllocVar<K::Native, Fr>,
{
    fn new_variable<T: std::borrow::Borrow<BaseCommitment<K::Native>>>(
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
        let key: K = variable(cs.clone(), &value.key, mode)?;
        let random = variable(cs.clone(), &value.random, mode)?;

        Ok(Self {
            asset,
            amount,
            spendability_hash,
            key,
            random,
        })
    }
}

impl AllocVar<NullifierKey, Fr> for NullifierKeyVar {
    fn new_variable<T: std::borrow::Borrow<NullifierKey>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let var = variable(cs, &value.0, mode)?;
        Ok(NullifierKeyVar(var))
    }
}

impl AllocVar<NullifierPubKey, Fr> for NullifierPubKeyVar {
    fn new_variable<T: std::borrow::Borrow<NullifierPubKey>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let var = variable(cs, &value.0, mode)?;
        Ok(NullifierPubKeyVar(var))
    }
}

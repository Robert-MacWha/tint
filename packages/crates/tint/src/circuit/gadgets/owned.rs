use ark_bn254::Fr;
use ark_r1cs_std::{
    alloc::AllocVar, eq::EqGadget, fields::FieldVar, prelude::Boolean, uint128::UInt128,
};
use ark_relations::gr1cs::SynthesisError;
use tint_poseidon2::poseidon2_compress_gadget;

use crate::{
    circuit::{FrVar, variable},
    fr::address_to_fr,
    note::owned::OwnedNote,
};

/// Circuit variable for a [`OwnedNote`].
#[derive(Clone, Debug)]
pub struct OwnedNoteVar {
    pub asset: FrVar,
    pub amount: UInt128<Fr>,
    pub policy_address: FrVar,
    pub policy_witness: FrVar,
    pub nullifier_key: FrVar,
    pub random: FrVar,
}

impl OwnedNoteVar {
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

    /// Computes the nullifier from this note's precomputed `hash`.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn nullifier(&self, hash: &FrVar) -> Result<FrVar, SynthesisError> {
        self.is_sentinel()?.select(
            &FrVar::zero(),
            &poseidon2_compress_gadget(&[self.nullifier_key.clone(), hash.clone()])?,
        )
    }

    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn is_sentinel(&self) -> Result<Boolean<Fr>, SynthesisError> {
        self.amount.is_eq(&UInt128::constant(0u128))
    }

    #[tracing::instrument(target = "r1cs", skip_all)]
    fn partial_hash(&self) -> Result<FrVar, SynthesisError> {
        poseidon2_compress_gadget(&[
            self.policy_hash()?,
            self.nullifying_pub_key()?,
            self.random.clone(),
        ])
    }

    #[tracing::instrument(target = "r1cs", skip_all)]
    fn policy_hash(&self) -> Result<FrVar, SynthesisError> {
        poseidon2_compress_gadget(&[self.policy_address.clone(), self.policy_witness.clone()])
    }

    #[tracing::instrument(target = "r1cs", skip_all)]
    fn nullifying_pub_key(&self) -> Result<FrVar, SynthesisError> {
        #[expect(clippy::cloned_ref_to_slice_refs)]
        poseidon2_compress_gadget(&[self.nullifier_key.clone()])
    }
}

impl AllocVar<OwnedNote, Fr> for OwnedNoteVar {
    fn new_variable<T: std::borrow::Borrow<OwnedNote>>(
        cs: impl Into<ark_relations::gr1cs::Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: ark_r1cs_std::prelude::AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let owned = value.borrow();
        let note = owned.note;

        let asset = variable(cs.clone(), &note.asset_fr(), mode)?;
        let amount = variable(cs.clone(), &note.amount, mode)?;
        let random = variable(cs.clone(), &note.random_fr(), mode)?;
        let nullifier_key = variable(cs.clone(), &owned.nullifier_key.0, mode)?;
        let policy_address = variable(cs.clone(), &address_to_fr(owned.policy.address), mode)?;
        let policy_witness = variable(cs.clone(), &owned.policy.witness, mode)?;

        Ok(Self {
            asset,
            amount,
            policy_address,
            policy_witness,
            nullifier_key,
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

        let owned_note = OwnedNote::default();
        let owned_note_var: OwnedNoteVar = witness(cs.clone(), &owned_note).unwrap();

        let owned_note_hash = owned_note.note.hash();
        let owned_note_var_hash = owned_note_var.hash().unwrap().value().unwrap();

        assert_eq!(owned_note_hash, owned_note_var_hash);
    }

    #[test]
    fn nullifier() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let mut owned_note = OwnedNote::default();
        owned_note.note.amount = 100;
        let owned_note_var: OwnedNoteVar = witness(cs.clone(), &owned_note).unwrap();

        let owned_note_nullifier = owned_note.nullifier();
        let owned_note_var_nullifier = owned_note_var
            .nullifier(&owned_note_var.hash().unwrap())
            .unwrap()
            .value()
            .unwrap();

        assert_eq!(owned_note_nullifier, owned_note_var_nullifier);
        assert_ne!(owned_note_nullifier, Fr::ZERO);
    }

    #[test]
    fn nullifier_sentinel() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let owned_note = OwnedNote::default();
        let owned_note_var: OwnedNoteVar = witness(cs.clone(), &owned_note).unwrap();

        let owned_note_nullifier = owned_note.nullifier();
        let owned_note_var_nullifier = owned_note_var
            .nullifier(&owned_note_var.hash().unwrap())
            .unwrap()
            .value()
            .unwrap();

        assert_eq!(owned_note_nullifier, owned_note_var_nullifier);
        assert_eq!(owned_note_nullifier, Fr::ZERO);
    }

    #[test]
    fn is_sentinel() {
        let cs = ConstraintSystem::<Fr>::new_ref();

        let sentinel = OwnedNote::default();
        let mut live = OwnedNote::default();
        live.note.amount = 100;

        let sentinel_var: OwnedNoteVar = witness(cs.clone(), &sentinel).unwrap();
        let live_var: OwnedNoteVar = witness(cs.clone(), &live).unwrap();

        assert!(sentinel_var.is_sentinel().unwrap().value().unwrap());
        assert!(!live_var.is_sentinel().unwrap().value().unwrap());
    }
}

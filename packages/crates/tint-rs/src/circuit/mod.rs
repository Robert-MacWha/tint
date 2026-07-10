use ark_r1cs_std::alloc::AllocVar;

pub mod join_split;
pub mod merkle_tree_inclusion;
pub mod merkle_tree_root;
pub mod merkle_tree_subtree_append;
pub mod operation;
pub mod poseidon;

pub type FrVar = ark_r1cs_std::fields::fp::FpVar<ark_bn254::Fr>;

/// Creates a new variable in the constraint system with the given value and allocation mode.
fn variable(
    cs: impl Into<ark_relations::gr1cs::Namespace<ark_bn254::Fr>>,
    value: ark_bn254::Fr,
    mode: ark_r1cs_std::prelude::AllocationMode,
) -> Result<FrVar, ark_relations::gr1cs::SynthesisError> {
    FrVar::new_variable(cs, || Ok(value), mode)
}

/// Creates an array of size N by trying to call the provided fn for each index.
fn try_array_from_fn<T, E, const N: usize>(
    mut f: impl FnMut(usize) -> Result<T, E>,
) -> Result<[T; N], E> {
    Ok((0..N)
        .map(&mut f)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .unwrap_or_else(|_| unreachable!()))
}

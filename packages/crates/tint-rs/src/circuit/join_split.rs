use std::borrow::Borrow;

use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use ark_r1cs_std::{
    GR1CSVar,
    alloc::{AllocVar, AllocationMode},
    eq::EqGadget,
    fields::FieldVar,
};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, Namespace, OptimizationGoal,
    SynthesisError,
};

use crate::{
    circuit::{
        FrVar, input,
        merkle_tree::{InclusionProofVar, SubtreeAppendProofVar},
        operation::OperationVar,
        output, try_array_from_fn, variable, witness,
    },
    indexer::merkle_tree::{InclusionProof, SubtreeAppendProof},
    operation::Operation,
};

pub const N_INPUTS: usize = 5;
pub const N_OUTPUTS: usize = 5;
pub const N_WITHDRAWALS: usize = 5;

/// Number of Groth16 public signals: old_root, old_root_length,
/// start_aggregation_hash, bound_params_hash, new_root, end_aggregation_hash,
/// nullifiers, output_commitment_hashes, (withdrawal_amount, withdrawal_asset)
/// interleaved per withdrawal slot. Mirrors `Constants.sol`'s `N_PUB`.
pub const N_PUB: usize = 4 + 2 + N_INPUTS + N_OUTPUTS + 2 * N_WITHDRAWALS;

pub const TREE_DEPTH: usize = 8;
pub const SUBTREE_DEPTH: usize = 2;
pub const SUBTREE_PATH_LENGTH: usize = TREE_DEPTH - SUBTREE_DEPTH;
pub const K: usize = 8;

pub const SUBTREE_SIZE: usize = K.pow(SUBTREE_DEPTH as u32);

#[derive(Clone, Default)]
pub struct JoinSplit {
    // Public inputs
    pub old_root: Fr,
    pub old_root_length: u64,
    pub start_aggregation_hash: Fr,
    pub bound_params_hash: Fr,

    // Witnessed values
    pub subtree_append: SubtreeAppendProof<SUBTREE_PATH_LENGTH, SUBTREE_SIZE, K>,
    pub commitment_inclusion_proofs: [InclusionProof<TREE_DEPTH, K>; N_INPUTS],
    pub operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct JoinSplitVar {
    pub subtree_append: SubtreeAppendProofVar<SUBTREE_PATH_LENGTH, SUBTREE_DEPTH, SUBTREE_SIZE, K>,
    pub commitment_inclusion_proofs: [InclusionProofVar<TREE_DEPTH, K>; N_INPUTS],
    pub operation: OperationVar<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
}

pub struct JoinSplitResult {
    pub new_root: Fr,
    pub end_aggregation_hash: Fr,
    pub nullifiers: [Fr; N_INPUTS],
    pub output_commitment_hashes: [Fr; N_OUTPUTS],
    pub withdrawal_amounts: [u128; N_WITHDRAWALS],
    pub withdrawal_assets: [Address; N_WITHDRAWALS],
}

pub struct JoinSplitResultVar {
    pub new_root: FrVar,
    pub end_aggregation_hash: FrVar,
    pub nullifiers: [FrVar; N_INPUTS],
    pub output_commitment_hashes: [FrVar; N_OUTPUTS],
    pub withdrawal_amounts: [FrVar; N_WITHDRAWALS],
    pub withdrawal_assets: [FrVar; N_WITHDRAWALS],
}

impl JoinSplit {
    /// Synthesizes the JoinSplit circuit, returning the public inputs (in
    /// the order matching `ProofLib.toPublicSignals` / `N_PUB`).
    pub fn synthesize_public_inputs(&self) -> Result<Vec<Fr>, SynthesisError> {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);

        let _ = self.synthesize(cs.clone())?;
        cs.finalize();

        // `instance_assignment()` leads with the implicit constant-1 term;
        // callers (and `Groth16::verify`) only want the actual signals.
        Ok(cs.instance_assignment()?[1..].to_vec())
    }

    /// Synthesizes the JoinSplit circuit, returning the public outputs.
    pub fn synthesize_outputs(&self) -> Result<JoinSplitResult, SynthesisError> {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);

        let result = self.synthesize(cs.clone())?;
        cs.finalize();

        result.try_into()
    }

    fn synthesize(
        &self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<JoinSplitResultVar, SynthesisError> {
        // Public inputs
        let old_root = input(cs.clone(), &self.old_root)?;
        let old_root_length = input(cs.clone(), &Fr::from(self.old_root_length))?;
        let start_aggregation_hash = input(cs.clone(), &self.start_aggregation_hash)?;
        let _bound_params_hash: FrVar = input(cs.clone(), &self.bound_params_hash)?;

        // Witnessed values
        let join_split_var: JoinSplitVar = witness(cs.clone(), self)?;

        let result = join_split_var.verify(&old_root, &old_root_length, &start_aggregation_hash)?;

        // Public outputs
        output(cs.clone(), &result.new_root)?;
        output(cs.clone(), &result.end_aggregation_hash)?;
        for i in 0..N_INPUTS {
            output(cs.clone(), &result.nullifiers[i])?;
        }
        for i in 0..N_OUTPUTS {
            output(cs.clone(), &result.output_commitment_hashes[i])?;
        }
        for i in 0..N_WITHDRAWALS {
            output(cs.clone(), &result.withdrawal_amounts[i])?;
            output(cs.clone(), &result.withdrawal_assets[i])?;
        }

        Ok(result)
    }
}

impl ConstraintSynthesizer<Fr> for JoinSplit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let _ = self.synthesize(cs)?;
        Ok(())
    }
}

impl JoinSplitVar {
    /// Verifies the JoinSplit operation.
    #[tracing::instrument(target = "r1cs", skip_all)]
    pub fn verify(
        &self,
        old_root: &FrVar,
        old_root_length: &FrVar,
        start_aggregation_hash: &FrVar,
    ) -> Result<JoinSplitResultVar, SynthesisError> {
        // Verify the staged leaf append proof and return the new root of the Merkle tree.
        let subtree_append_result =
            self.subtree_append
                .verify(old_root, old_root_length, start_aggregation_hash)?;
        let new_root = subtree_append_result.new_root;

        // Verify the inclusion proofs for the input commitments. Skipped for
        // zero-valued leaves (padding).
        for proof in &self.commitment_inclusion_proofs {
            let implied_root = proof.root()?;
            let used = !proof.leaf.is_zero()?;

            let expected = used.select(&new_root, &implied_root)?;
            implied_root.enforce_equal(&expected)?;
        }

        let input_commitment_hashes =
            &std::array::from_fn(|i| self.commitment_inclusion_proofs[i].leaf.clone());

        // Verify that the operation is balanced and returns the resulting outputs.
        let operation_result = self.operation.verify(input_commitment_hashes)?;

        Ok(JoinSplitResultVar {
            new_root,
            end_aggregation_hash: subtree_append_result.end_aggregation_hash,
            nullifiers: operation_result.nullifiers,
            output_commitment_hashes: operation_result.output_commitment_hashes,
            withdrawal_amounts: std::array::from_fn(|i| {
                operation_result.withdrawals[i].amount.clone()
            }),
            withdrawal_assets: std::array::from_fn(|i| {
                operation_result.withdrawals[i].asset.clone()
            }),
        })
    }
}

impl AllocVar<JoinSplit, Fr> for JoinSplitVar {
    fn new_variable<T: Borrow<JoinSplit>>(
        cs: impl Into<Namespace<Fr>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let cs = cs.into();
        let value = f()?;
        let value = value.borrow();

        let subtree_append = variable(cs.clone(), &value.subtree_append, mode)?;
        let commitment_inclusion_proofs = try_array_from_fn(|i| {
            variable(cs.clone(), &value.commitment_inclusion_proofs[i], mode)
        })?;
        let operation = variable(cs.clone(), &value.operation, mode)?;

        Ok(Self {
            subtree_append,
            commitment_inclusion_proofs,
            operation,
        })
    }
}

impl TryFrom<JoinSplitResultVar> for JoinSplitResult {
    type Error = SynthesisError;

    fn try_from(value: JoinSplitResultVar) -> Result<Self, Self::Error> {
        let new_root = value.new_root.value()?;
        let end_aggregation_hash = value.end_aggregation_hash.value()?;
        let nullifiers = try_array_from_fn(|i| value.nullifiers[i].value())?;
        let output_commitment_hashes =
            try_array_from_fn(|i| value.output_commitment_hashes[i].value())?;
        let withdrawal_amounts: [Fr; N_WITHDRAWALS] =
            try_array_from_fn(|i| value.withdrawal_amounts[i].value())?;
        let withdrawal_assets: [Fr; N_WITHDRAWALS] =
            try_array_from_fn(|i| value.withdrawal_assets[i].value())?;

        let withdrawal_amounts = try_array_from_fn(|i| fr_to_u128(&withdrawal_amounts[i]))?;
        let withdrawal_assets = try_array_from_fn(|i| fr_to_address(&withdrawal_assets[i]))?;

        Ok(Self {
            new_root,
            end_aggregation_hash,
            nullifiers,
            output_commitment_hashes,
            withdrawal_amounts,
            withdrawal_assets,
        })
    }
}

fn fr_to_u128(fr: &Fr) -> Result<u128, SynthesisError> {
    let bytes = fr.into_bigint().to_bytes_le();
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes[..16]);
    Ok(u128::from_le_bytes(arr))
}

fn fr_to_address(fr: &Fr) -> Result<alloy_primitives::Address, SynthesisError> {
    let bytes = fr.into_bigint().to_bytes_le();
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes[..20]);
    Ok(alloy_primitives::Address::from(arr))
}

// #[cfg(test)]
// mod tests {
//     use alloy_primitives::{Address, B256};
//     use ark_bn254::Bn254;
//     use ark_ff::UniformRand;
//     use ark_groth16::Groth16;
//     use ark_snark::SNARK;
//     use ark_std::rand::{Rng, SeedableRng, rngs::StdRng};

//     use super::*;
//     use crate::{
//         circuit::poseidon::poseidon_hash,
//         indexer::merkle_tree::IncrementalMerkleTree,
//         note::{
//             asset::AssetId,
//             commitment::{BaseCommitment, Commitment, NullifierKey},
//         },
//     };

//     /// Expect that a JoinSplit spending one pre-existing note into one new
//     /// note produces a proof that a real Groth16 setup/prove/verify round
//     /// trip accepts, and that a tampered public input is rejected.
//     #[test]
//     fn join_split_circuit_proves_and_verifies_a_transfer() {
//         let mut rng = StdRng::seed_from_u64(42);

//         let sender_key = NullifierKey(Fr::rand(&mut rng));
//         let recipient_key = NullifierKey(Fr::rand(&mut rng));

//         let input_commitment = crate::note::commitment::SpendableCommitment::new(
//             BaseCommitment::new(
//                 Address::repeat_byte(0xaa),
//                  100,
//                   Address::ZERO,
//                    B256::ZERO,
//                     sender_key,
//                      B256::new(rng.r#gen())
//                     ),

//         );

//         let mut tree = IncrementalMerkleTree::<TREE_DEPTH, K>::new();
//         tree.append(&[input_commitment.hash()]);
//         let old_root = tree.root();
//         let old_root_length = 1u64;

//         let output_commitment = Commitment::new(
//             input_commitment.asset,
//             input_commitment.amount,
//             Address::ZERO,
//             Default::default(),
//             recipient_key.pub_key(),
//             Fr::rand(&mut rng),
//         );
//         let output_hash = output_commitment.hash();

//         let subtree_append = tree
//             .append_subtree::<SUBTREE_PATH_LENGTH, SUBTREE_SIZE>(&[output_hash])
//             .unwrap();
//         let new_root = tree.root();

//         // The input's inclusion proof must be taken against the tree's
//         // post-append state, since `JoinSplitVar::verify` checks input
//         // membership against `new_root` (not `old_root`) — and appending the
//         // output leaf changes sibling hashes shared with the input's path.
//         let inclusion_path = tree.path(input_commitment.hash()).unwrap();
//         let input_inclusion_proof = tree.inclusion(inclusion_path);

//         let start_aggregation_hash = Fr::from(0u64);
//         let end_aggregation_hash = poseidon_hash(&[start_aggregation_hash, output_hash]);

//         let dummy_inclusion_proof = InclusionProof {
//             path: [0u8; TREE_DEPTH],
//             siblings: [[Fr::from(0u64); K]; TREE_DEPTH],
//             leaf: Fr::from(0u64),
//         };
//         let commitment_inclusion_proofs = std::array::from_fn(|i| {
//             if i == 0 {
//                 input_inclusion_proof.clone()
//             } else {
//                 dummy_inclusion_proof.clone()
//             }
//         });

//         let mut operation = Operation::<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>::default();
//         operation.inputs[0] = input_commitment.clone();
//         operation.output_commitments[0] = output_commitment;

//         let witness = JoinSplit {
//             subtree_append,
//             commitment_inclusion_proofs,
//             operation,
//         };

//         let mut nullifiers = [Fr::from(0u64); N_INPUTS];
//         nullifiers[0] = input_commitment.nullifier();
//         let mut output_commitment_hashes = [Fr::from(0u64); N_OUTPUTS];
//         output_commitment_hashes[0] = output_hash;
//         // Dummy withdrawal slots aren't gated to zero (only `amount == 0`
//         // marks them unused) — their `asset` reveals whatever `AssetId::default()`
//         // hashes to, since AssetId::to_fr() is a keccak hash and never literally 0.
//         let unshield_assets =
//             [crate::note::withdrawal::Withdrawal::default().asset_fr(); N_OUTPUTS];

//         let circuit = JoinSplitCircuit {
//             witness,
//             old_root_length,
//             start_aggregation_hash,
//             old_root,
//             new_root,
//             end_aggregation_hash,
//             nullifiers,
//             output_commitment_hashes,
//             unshield_amounts: [Fr::from(0u64); N_OUTPUTS],
//             unshield_assets,
//             bound_params_hash: Fr::from(0u64),
//         };

//         let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), &mut rng).unwrap();

//         let public_inputs: Vec<Fr> = [old_root, new_root, end_aggregation_hash]
//             .into_iter()
//             .chain(nullifiers)
//             .chain(output_commitment_hashes)
//             .chain([Fr::from(0u64); N_OUTPUTS]) // unshield_amounts
//             .chain(unshield_assets)
//             .chain([Fr::from(0u64)]) // bound_params_hash
//             .collect();

//         let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).unwrap();
//         assert!(Groth16::<Bn254>::verify(&vk, &public_inputs, &proof).unwrap());

//         let mut tampered_inputs = public_inputs.clone();
//         tampered_inputs[0] = Fr::from(999u64);
//         assert!(!Groth16::<Bn254>::verify(&vk, &tampered_inputs, &proof).unwrap());
//     }
// }

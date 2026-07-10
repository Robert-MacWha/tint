// use crate::circuits::inputs::{AggregatorProofInputs, CHUNK_SIZE, DEPTH};

// use alloy_primitives::{Address, B256, Bytes, U256};
// use alloy_sol_types::SolCall;
// use ark_bn254::Bn254;
// use ark_bn254::Fr;
// use ark_ff::{AdditiveGroup, BigInt, Field, PrimeField};
// use ark_groth16::Proof;
// use ark_groth16::ProvingKey;
// use circom_witness_rs::calculate_witness;
// use circom_witness_rs::{BlackBoxFunction, Graph};
// use std::collections::HashMap;
// use std::sync::Arc;

// use crate::{
//     abis::tint::IPrivacyPool, indexer::Indexer, note::commitment::Commitment, operation::Operation,
// };

// /// Provides an interface for interacting with the tint smart contract.
// ///
// /// Manages sync state internally, helps to create deposits and perform shielded
// /// operations.
// pub struct Provider {
//     indexer: Indexer,
//     witness_graph: Graph,
//     // matrices: NPIndex<Fr>,
//     proving_key: ProvingKey<Bn254>,
// }

// pub struct EthCall {
//     pub to: Address,
//     pub data: Bytes,
//     pub value: U256,
// }

// // #[derive(Debug, thiserror::Error)]
// pub enum ProviderError {}

// impl Provider {
//     pub fn new(
//         indexer: Indexer,
//         witness_graph: Graph,
//         // matrices: NPIndex<Fr>,
//         proving_key: ProvingKey<Bn254>,
//     ) -> Self {
//         Provider {
//             indexer,
//             witness_graph,
//             // matrices,
//             proving_key,
//         }
//     }

//     /// Creates a deposit function call for the given commitment.
//     pub fn deposit(&self, commitment: Commitment) -> IPrivacyPool::depositCall {
//         todo!()
//         // IPrivacyPool::depositCall::new((commitment.asset, commitment.amount, commitment.hash()))
//     }

//     /// Creates an operate function call for the given operation.
//     #[allow(non_snake_case)]
//     pub fn operate(
//         &mut self,
//         operation: Operation,
//         r: Fr,
//         s: Fr,
//     ) -> Result<IPrivacyPool::operateCall, ProviderError> {
//         let inputs = self.generate_inputs(operation);
//         let oldRoot = inputs.old_root;
//         let newRoot = inputs.new_root;
//         let leavesAggregationIndex = (inputs.chunk_insert_witness.chunk_index * CHUNK_SIZE as u64
//             + inputs.chunk_insert_witness.chunk_filled)
//             as u128;
//         let nullifiers = inputs.nullifiers.clone();
//         let commitmentsOut = inputs.commitments_out.clone();
//         let unshieldAmounts = inputs.unshield_amounts.clone();
//         let unshieldAssets = inputs.unshield_assets.clone();

//         todo!()

//         // let proof = self.generate_proof(inputs, r, s)?;
//         // let op = IPrivacyPool::Operation {
//         //     oldRoot,
//         //     newRoot,
//         //     leavesAggregationIndex,
//         //     nullifiers,
//         //     commitmentsOut,
//         //     unshieldAmounts,
//         //     unshieldAssets,
//         //     unshieldRecipients: [Address::ZERO; 5],    // TODO
//         //     spendabilityAddresses: [Address::ZERO; 5], // TODO
//         //     spendabilityData: [
//         //         Bytes::new(),
//         //         Bytes::new(),
//         //         Bytes::new(),
//         //         Bytes::new(),
//         //         Bytes::new(),
//         //     ], // TODO
//         //     proof: proof.into(),
//         // };

//         // Ok(IPrivacyPool::operateCall::new((op,)))
//     }

//     fn generate_inputs(&mut self, operation: Operation) -> AggregatorProofInputs {
//         let old_root = self.indexer.root();
//         let start_aggregation_hash = self.indexer.aggregation_hash();

//         let chunk_insert_witness = self.indexer.commit();

//         let new_root = self.indexer.root();
//         let end_aggregation_hash = self.indexer.aggregation_hash();

//         AggregatorProofInputs {
//             old_root,
//             new_root,
//             start_aggregation_hash,
//             end_aggregation_hash,
//             nullifiers: operation.nullifiers(),
//             commitments_out: operation.commitment_hashes(),
//             unshield_amounts: operation.unshield_amounts(),
//             unshield_assets: operation.unshield_assets(),
//             bound_params_hash: B256::ZERO, // TODO
//             chunk_insert_witness,
//             leaf_indices_in: operation.leaf_indices_in(),
//             siblings_in: std::array::from_fn(|i| {
//                 self.indexer
//                     .prove(operation.inputs[i].leaf_index)
//                     .unwrap_or([B256::ZERO; DEPTH])
//             }),
//             assets_in: operation.assets_in(),
//             amounts_in: operation.amounts_in(),
//             nullifying_keys_in: operation.nullifying_keys(),
//             spendability_hashes_in: operation.spendability_hashes_in(),
//             random_in: operation.random_in(),
//             assets_out: operation.commitment_assets(),
//             amounts_out: operation.commitment_amounts(),
//             spendability_hashes_out: operation.spendability_hashes_out(),
//             nullifying_pub_keys_out: operation.nullifying_pub_keys_out(),
//             random_out: operation.random_out(),
//         }
//     }

//     fn generate_proof(
//         &self,
//         inputs: AggregatorProofInputs,
//         r: Fr,
//         s: Fr,
//     ) -> Result<Proof<Bn254>, ProviderError> {
//         // let input_list = inputs.inputs_list();

//         // let bbfs: HashMap<String, BlackBoxFunction> = HashMap::from([
//         //     ("bbf_inv".to_string(), make_bbf(bbf_inv)),
//         //     ("bbf_bit".to_string(), make_bbf(bbf_bit)),
//         // ]);
//         // let witness = calculate_witness(input_list, &self.witness_graph, Some(&bbfs)).unwrap();
//         // let witness: Vec<Fr> = witness
//         //     .into_iter()
//         //     .map(|x| Fr::from(BigInt::from(x.clone())))
//         //     .collect();

//         // let proof = Groth16::<Bn254, CircomReduction>::create_proof_with_reduction_and_matrices(
//         //     &self.proving_key,
//         //     r,
//         //     s,
//         //     &[self.matrices.a.clone(), self.matrices.b.clone()],
//         //     self.matrices.num_instance_variables,
//         //     self.matrices.num_constraints,
//         //     &witness,
//         // )
//         // .unwrap();

//         // Ok(proof)
//         todo!()
//     }
// }

// fn make_bbf<F>(f: F) -> BlackBoxFunction
// where
//     F: Fn(&[Fr]) -> Fr + Send + Sync + 'static,
// {
//     Arc::new(f)
// }

// fn bbf_inv(args: &[Fr]) -> Fr {
//     args[0].inverse().unwrap_or(Fr::ZERO)
// }

// fn bbf_bit(args: &[Fr]) -> Fr {
//     let val = args[0].into_bigint();
//     let shift = args[1].into_bigint().0[0] as usize;
//     Fr::from((val.0[shift / 64] >> (shift % 64)) & 1)
// }

// fn b256_arr<const N: usize>(v: &[[u8; 32]]) -> [B256; N] {
//     std::array::from_fn(|i| B256::from(v.get(i).copied().unwrap_or([0u8; 32])))
// }

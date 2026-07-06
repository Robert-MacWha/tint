use crate::circuits::inputs::AggregatorProofInputs;
use crate::circuits::inputs::DEPTH;
use crate::circuits::inputs::N_INPUTS;

use alloy::{
    primitives::{Address, B256, Bytes, U256},
    sol_types::SolCall,
};
use ark_bn254::Bn254;
use ark_bn254::Fr;
use ark_circom::CircomReduction;
use ark_circom::index::NPIndex;
use ark_ff::BigInt;
use ark_groth16::Groth16;
use ark_groth16::ProvingKey;
use circom_witness_rs::Graph;
use circom_witness_rs::calculate_witness;

use crate::{
    abis::tint::IPrivacyPool, indexer::Indexer, note::commitment::Commitment, operation::Operation,
};

/// Provides an interface for interacting with the tint smart contract.
///
/// Manages sync state internally, helps to create deposits and perform shielded
/// operations.
pub struct Provider {
    indexer: Indexer,
    witness_graph: Graph,
    matrices: NPIndex<Fr>,
    proving_key: ProvingKey<Bn254>,
}

pub struct EthCall {
    pub to: Address,
    pub data: Bytes,
    pub value: U256,
}

// #[derive(Debug, thiserror::Error)]
pub enum ProviderError {}

impl Provider {
    pub fn new(
        indexer: Indexer,
        witness_graph: Graph,
        matrices: NPIndex<Fr>,
        proving_key: ProvingKey<Bn254>,
    ) -> Self {
        Provider {
            indexer,
            witness_graph,
            matrices,
            proving_key,
        }
    }

    /// Creates a deposit function call for the given commitment.
    pub fn deposit(&self, commitment: Commitment) -> IPrivacyPool::depositCall {
        IPrivacyPool::depositCall::new((commitment.asset, commitment.amount, commitment.hash()))
    }

    /// Creates an operate function call for the given operation.
    pub fn operate(
        &mut self,
        operation: Operation,
        r: Fr,
        s: Fr,
    ) -> Result<IPrivacyPool::operateCall, ProviderError> {
        let old_root = self.indexer.root();
        let start_aggregation_hash = self.indexer.aggregation_hash();
        let batch_start_index = self.indexer.leaves();
        let initial_frontier = self.indexer.frontier();

        let new_leaves = self.indexer.commit().unwrap();

        let new_root = self.indexer.root();
        let end_aggregation_hash = self.indexer.aggregation_hash();

        let aggregator_proof_inputs = AggregatorProofInputs {
            old_root,
            new_root,
            start_aggregation_hash,
            end_aggregation_hash,
            nullifiers: operation.nullifiers(),
            commitments_out: operation.commitment_hashes(),
            unshield_amounts: operation.unshield_amounts(),
            unshield_assets: operation.unshield_assets(),
            bound_params_hash: B256::ZERO, // TODO
            batch_start_index,
            new_leaves,
            initial_frontier,
            siblings_in: [[B256::ZERO; DEPTH]; N_INPUTS], // TODO
            leaf_indices_in: [0; N_INPUTS],               // TODO
            assets_in: operation.assets_in(),
            amounts_in: operation.amounts_in(),
            nullifying_keys_in: operation.nullifying_keys(),
            partial_commitments_in: operation.partial_commitments_in(),
            assets_out: operation.commitment_assets(),
            amounts_out: operation.commitment_amounts(),
            partial_commitments_out: operation.partial_commitments_out(),
        };

        let input_list = aggregator_proof_inputs.inputs_list();
        let witness = calculate_witness(input_list, &self.witness_graph, None).unwrap();
        let witness: Vec<Fr> = witness
            .into_iter()
            .map(|x| Fr::from(BigInt::from(x.clone())))
            .collect();

        let proof = Groth16::<Bn254, CircomReduction>::create_proof_with_reduction_and_matrices(
            &self.proving_key,
            r,
            s,
            &[self.matrices.a.clone(), self.matrices.b.clone()],
            self.matrices.num_instance_variables,
            self.matrices.num_constraints,
            &witness,
        )
        .unwrap();

        todo!()
    }
}

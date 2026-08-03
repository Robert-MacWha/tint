use std::array::repeat;

use alloy_primitives::{Address, B256, Bytes, keccak256};
use alloy_sol_types::{SolCall, SolValue};
use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_std::rand::Rng;
use rand_core::{CryptoRng, RngCore};
use tracing::info;

use crate::{
    abis::tint::{IPrivacyPool, Tint},
    account::{Account, keys::NullifierPubKey, receiver::Receiver},
    array::try_from_fn,
    circuit::{
        Artifacts,
        join_split::{JoinSplit, K, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, TREE_DEPTH},
        matrices::prove_with_matrices,
    },
    database::DatabaseError,
    fr::fr_to_b256,
    indexer::{Indexer, merkle_tree::InclusionProof},
    note::{
        asset::AssetId,
        commitment::{BaseCommitment, Commitment, NullifiableCommitment, SpendableCommitment},
        withdrawal::Withdrawal,
    },
    operation::Operation,
};

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("more inputs, outputs, or withdrawals than this operation supports")]
    TooManySlots,
    #[error("input commitment not present in the tree — not yet synced, or already spent")]
    InputNotFound,
    #[error("generated proof is invalid")]
    InvalidProof,
    #[error("indexer error: {0}")]
    Indexer(#[from] crate::indexer::IndexerError),
    #[error("merkle tree error: {0}")]
    MerkleTree(#[from] crate::indexer::merkle_tree::MerkleTreeError),
    #[error("circuit error: {0}")]
    Synthesis(#[from] ark_relations::gr1cs::SynthesisError),
    #[error("commitment error: {0}")]
    Commitment(#[from] crate::note::commitment::CommitmentError),
    #[error("spending account error: {0}")]
    Spending(#[from] crate::account::spending::SpendingAccountError),
    #[error(
        "no registered account authorizes spending note with nullifier pub key {nullifier_pub_key:?} and spendability hash {spendability_hash:?}"
    )]
    UnknownSpendingAccount {
        nullifier_pub_key: NullifierPubKey,
        spendability_hash: Fr,
    },
}

/// Builds shield/transfer/unshield calls against a Tint deployment.
pub struct Provider {
    pub indexer: Indexer,
    accounts: Vec<Account>,
    artifacts: Artifacts<Bn254>,
}

impl Provider {
    #[must_use]
    pub fn new(indexer: Indexer, artifacts: Artifacts<Bn254>) -> Self {
        Self {
            indexer,
            accounts: Vec::new(),
            artifacts,
        }
    }

    /// Adds an account which will be indexed and made available to authorize
    /// spending its own notes in [`Self::operate`]/[`Self::public_inputs`].
    pub async fn add_account(&mut self, account: Account) -> Result<(), DatabaseError> {
        self.indexer
            .add_account(account.viewing().clone(), account.nullifying().clone())
            .await?;
        self.accounts.push(account);
        Ok(())
    }

    /// Returns the notes owned by `receiver`.
    #[must_use]
    pub fn notes(&self, receiver: Receiver) -> Vec<&NullifiableCommitment> {
        self.indexer.notes(receiver)
    }

    /// Synchronize the indexer with the on-chain state.
    pub async fn sync(&mut self) -> Result<(), ProviderError> {
        self.indexer.sync().await?;
        Ok(())
    }

    /// Builds a `deposit` call for a new note payable to `receiver`.
    pub fn deposit<R: RngCore + CryptoRng>(
        &self,
        receiver: Receiver,
        asset: AssetId,
        amount: u128,
        rng: &mut R,
    ) -> Result<Tint::depositCall, ProviderError> {
        let random = B256::new(rng.r#gen());
        let commitment = receiver.commitment(asset, amount, random);
        let encrypted = commitment.encrypt(&[receiver.encryption_pub_key], rng)?;

        Ok(Tint::depositCall {
            asset: asset.into(),
            amount,
            partialCommitment: fr_to_b256(commitment.partial_hash()),
            encryptedNote: Bytes::from(encrypted),
        })
    }

    /// Builds a proven `operate` call spending `inputs` into `outputs`
    /// (new shielded notes) and `withdrawals` (unshields). Each input is
    /// resolved against the accounts registered via [`Self::add_account`].
    pub async fn operate<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: [NullifiableCommitment; I],
        outputs: [(Receiver, AssetId, u128); O],
        withdrawals: [(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<Tint::operateCall, ProviderError> {
        let (operation, _public_inputs) = self.operation(inputs, outputs, withdrawals, rng).await?;
        Ok(Tint::operateCall::new((operation,)))
    }

    /// Computes the public-input vector and on-chain `Operation` for
    /// this operation without generating a Groth16 proof.
    pub async fn public_inputs<
        const I: usize,
        const O: usize,
        const W: usize,
        R: RngCore + CryptoRng,
    >(
        &mut self,
        inputs: [NullifiableCommitment; I],
        outputs: [(Receiver, AssetId, u128); O],
        withdrawals: [(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<(Tint::computePublicSignalsCall, Vec<Fr>), ProviderError> {
        let (operation, public_inputs) = self.operation(inputs, outputs, withdrawals, rng).await?;

        Ok((
            Tint::computePublicSignalsCall::new((operation,)),
            public_inputs,
        ))
    }

    /// Computes the public-input vector and on-chain `Operation` for
    /// this operation.
    async fn operation<const I: usize, const O: usize, const W: usize, R: RngCore + CryptoRng>(
        &mut self,
        inputs: [NullifiableCommitment; I],
        outputs: [(Receiver, AssetId, u128); O],
        withdrawals: [(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<(IPrivacyPool::Operation, Vec<Fr>), ProviderError> {
        let (circuit, context) = self
            .build_circuit(inputs, &outputs, &withdrawals, rng)
            .await?;

        let old_root = circuit.old_root;
        let start_aggregation_index = circuit.start_aggregation_index;
        let end_aggregation_index = self.indexer.posted_aggregation_index();

        info!("Proving operation...");
        let outputs = circuit.synthesize_outputs()?;
        let (public_inputs, proof) =
            prove_with_matrices(&self.artifacts.matrices, &self.artifacts.pk, &circuit, rng)?;

        // Smoke-test the proof locally
        if !Groth16::<Bn254>::verify(&self.artifacts.vk, &public_inputs, &proof)? {
            return Err(ProviderError::InvalidProof);
        }
        info!("Operation proof verified");

        Ok((
            IPrivacyPool::Operation {
                oldRoot: fr_to_b256(old_root),
                startAggregationIndex: start_aggregation_index,
                endAggregationIndex: end_aggregation_index,
                newRoot: fr_to_b256(outputs.new_root),
                operationHash: fr_to_b256(outputs.operation_hash),
                nullifiers: outputs.nullifiers.map(fr_to_b256),
                commitmentsOut: outputs.output_commitment_hashes.map(fr_to_b256),
                unshieldAmounts: outputs.withdrawal_amounts,
                unshieldAssets: outputs.withdrawal_assets.map(|a| a.0),
                spendabilityAddresses: outputs.spendability_addresses,
                context,
                proof: proof.into(),
            },
            public_inputs,
        ))
    }

    /// Builds the `JoinSplit` circuit witnessing `inputs` spent into
    /// `outputs` + `withdrawals`.
    async fn build_circuit<
        const I: usize,
        const O: usize,
        const W: usize,
        R: RngCore + CryptoRng,
    >(
        &mut self,
        inputs: [NullifiableCommitment; I],
        outputs: &[(Receiver, AssetId, u128); O],
        withdrawals: &[(Address, AssetId, u128); W],
        rng: &mut R,
    ) -> Result<(JoinSplit, IPrivacyPool::Context), ProviderError> {
        let old_root = self.indexer.root();
        let start_aggregation_index = self.indexer.posted_aggregation_index();
        let start_aggregation_hash = self.indexer.posted_aggregation_hash();

        let subtree_append = self.indexer.commit()?;

        let (output_commitments, output_withdrawals) = build_outputs(outputs, withdrawals, rng);

        let placeholder_inputs: [SpendableCommitment; I] =
            inputs.map(|note| note.as_pending_spendable());
        let mut operation =
            assemble_operation(&placeholder_inputs, output_commitments, output_withdrawals);

        let mut resolved_inputs: [SpendableCommitment; I] = repeat(SpendableCommitment::default());
        for (i, note) in inputs.iter().enumerate() {
            let account = self.account_for(note)?;
            let resolved = account
                .spending()
                .as_spendable(*note, operation.clone())
                .await?;
            operation.inputs[i] = resolved.clone();
            resolved_inputs[i] = resolved;
        }

        let commitment_inclusion_proofs = self.commitment_inclusion_proofs(&resolved_inputs)?;

        let unshield_recipients = unshield_recipients(withdrawals);
        let spendability_inputs = spendability_inputs(&resolved_inputs);
        let ciphertexts = try_from_fn(|i| {
            let output = &operation.output_commitments[i];
            let Some((receiver, _, _)) = outputs.get(i) else {
                return Ok::<Bytes, ProviderError>(Bytes::new());
            };

            // TODO: We'd ideally encrypt with both the sender and receiver's keys. The issue
            // is a single operation may have multiple senders, so we don't have a single key to use.
            // We could add a list of keys as args to this function, I'm not sure if there's a better
            // solution.
            Ok(Bytes::from(
                output.encrypt(&[receiver.encryption_pub_key], rng)?,
            ))
        })?;
        let context = IPrivacyPool::Context {
            spendabilityInputs: spendability_inputs,
            unshieldRecipients: unshield_recipients,
            ciphertexts,
        };
        let bound_params_hash = bound_params_hash(&context);

        let circuit = JoinSplit::new(
            // Public inputs
            old_root,
            start_aggregation_index,
            start_aggregation_hash,
            bound_params_hash,
            // Witnessed values
            subtree_append,
            commitment_inclusion_proofs,
            operation,
        );

        Ok((circuit, context))
    }

    /// Finds the registered account whose viewing/spending identity matches `note`.
    fn account_for(&self, note: &NullifiableCommitment) -> Result<&Account, ProviderError> {
        self.accounts
            .iter()
            .find(|account| {
                account.nullifying().pub_key() == note.inner.nullifier_pub_key
                    && account.spending().spendability_hash() == note.spendability_hash()
            })
            .ok_or(ProviderError::UnknownSpendingAccount {
                nullifier_pub_key: note.inner.nullifier_pub_key,
                spendability_hash: note.spendability_hash(),
            })
    }

    /// Returns the inclusion proofs for each of the given `inputs` in the current tree.
    fn commitment_inclusion_proofs<const I: usize>(
        &self,
        inputs: &[SpendableCommitment; I],
    ) -> Result<[InclusionProof<{ TREE_DEPTH }, { K }>; N_INPUTS], ProviderError> {
        let mut commitment_inclusion_proofs = repeat(InclusionProof::default());
        for (i, input) in inputs.iter().enumerate() {
            let proof = self
                .indexer
                .prove(input.hash())
                .ok_or(ProviderError::InputNotFound)?;
            commitment_inclusion_proofs[i] = proof;
        }

        Ok(commitment_inclusion_proofs)
    }
}

/// Builds the padded (real) output commitments and withdrawals for an
/// operation, consuming `rng`. Called exactly once per `operate`/`public_inputs`
/// call so a draft and its final operation always share the same outputs.
fn build_outputs<const O: usize, const W: usize, R: RngCore + CryptoRng>(
    outputs: &[(Receiver, AssetId, u128); O],
    withdrawals: &[(Address, AssetId, u128); W],
    rng: &mut R,
) -> ([BaseCommitment; N_OUTPUTS], [Withdrawal; N_WITHDRAWALS]) {
    const {
        assert!(O <= N_OUTPUTS, "too many outputs");
        assert!(W <= N_WITHDRAWALS, "too many withdrawals");
    }

    let mut output_commitments = repeat(BaseCommitment::default());
    for (i, (receiver, asset, amount)) in outputs.iter().enumerate() {
        let random = B256::new(rng.r#gen());
        output_commitments[i] = receiver.commitment(*asset, *amount, random);
    }

    let mut output_withdrawals = repeat(Withdrawal::default());
    for (i, (_, asset, amount)) in withdrawals.iter().enumerate() {
        output_withdrawals[i] = Withdrawal::new(*asset, *amount);
    }

    (output_commitments, output_withdrawals)
}

/// Pads `inputs` up to the circuit's fixed slot count and assembles an `Operation`
/// from them and already-built outputs/withdrawals.
#[allow(clippy::large_types_passed_by_value)]
fn assemble_operation<const I: usize>(
    inputs: &[SpendableCommitment; I],
    output_commitments: [BaseCommitment; N_OUTPUTS],
    output_withdrawals: [Withdrawal; N_WITHDRAWALS],
) -> Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> {
    const {
        assert!(I <= N_INPUTS, "too many inputs");
    }

    let mut input_commitments = repeat(SpendableCommitment::default());
    for (i, input) in inputs.iter().enumerate() {
        input_commitments[i] = input.clone();
    }

    Operation::new(input_commitments, output_commitments, output_withdrawals)
}

fn spendability_inputs<const I: usize>(inputs: &[SpendableCommitment; I]) -> [Bytes; N_INPUTS] {
    let mut spendability_inputs = repeat(Bytes::new());
    for (i, input) in inputs.iter().enumerate() {
        spendability_inputs[i] = input.spendability_input.clone();
    }
    spendability_inputs
}

fn unshield_recipients<const W: usize>(
    withdrawals: &[(Address, AssetId, u128); W],
) -> [Address; N_WITHDRAWALS] {
    let mut unshield_recipients = repeat(Address::ZERO);
    for (i, (addr, _, _)) in withdrawals.iter().enumerate() {
        unshield_recipients[i] = *addr;
    }
    unshield_recipients
}

/// Mirrors `ProofLib.toBoundParamsHash`
fn bound_params_hash(context: &IPrivacyPool::Context) -> Fr {
    Fr::from_be_bytes_mod_order(keccak256(context.abi_encode()).as_slice())
}

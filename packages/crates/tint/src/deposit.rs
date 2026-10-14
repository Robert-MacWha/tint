use alloy_primitives::{Address, B256, U256};
use alloy_provider::network::TransactionBuilder;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_sol_types::SolCall;
use ark_std::rand::Rng;
use rand_core::{CryptoRng, RngCore};

use crate::{
    abis::erc20::ERC20,
    account::pub_account::PubAccount,
    fr::fr_to_b256,
    note::{
        asset::AssetId,
        commitment::{BaseCommitment, Commitment},
    },
};

pub struct Deposit {
    pub instance: Address,
    pub receiver: PubAccount,
    pub asset: AssetId,
    pub amount: u128,
    pub random: B256,
}

#[derive(Debug, thiserror::Error)]
pub enum DepositError {
    #[error("commitment error: {0}")]
    Commitment(#[from] crate::note::commitment::CommitmentError),
}

impl Deposit {
    pub fn new<R: RngCore + CryptoRng>(
        instance: Address,
        receiver: PubAccount,
        asset: AssetId,
        amount: u128,
        rng: &mut R,
    ) -> Self {
        Self {
            instance,
            receiver,
            asset,
            amount,
            random: B256::new(rng.r#gen()),
        }
    }

    /// The note this deposit creates.
    #[must_use]
    pub fn note(&self) -> BaseCommitment {
        self.receiver
            .commitment(self.asset, self.amount, self.random)
    }

    #[must_use]
    pub fn approval(&self) -> TransactionRequest {
        let approval_call = ERC20::approveCall {
            spender: self.instance,
            amount: U256::from(self.amount),
        };

        TransactionRequest::default()
            .with_to(self.asset.into())
            .input(approval_call.abi_encode().into())
    }

    pub fn request<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
    ) -> Result<TransactionRequest, DepositError> {
        let commitment = self.note();
        let encrypted = commitment
            .partial()
            .encrypt(&[self.receiver.encryption_pub_key], rng)?;

        let deposit_call = crate::abis::tint::Tint::depositCall {
            asset: self.asset.into(),
            amount: self.amount,
            partialCommitment: fr_to_b256(commitment.partial_hash()),
            encryptedPartial: encrypted.into(),
        };

        Ok(TransactionRequest::default()
            .with_to(self.instance)
            .input(deposit_call.abi_encode().into()))
    }
}

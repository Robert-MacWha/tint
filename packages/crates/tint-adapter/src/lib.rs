//! Client for relaying Tint operations through a `TintAdapter` to dapps.

use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use alloy_rpc_types_eth::{TransactionInput, TransactionRequest};
use alloy_sol_types::SolValue;
use rand_core::{CryptoRng, RngCore};
use tint::{
    account::pub_account::PubAccount,
    fr::fr_to_b256,
    note::{asset::AssetId, commitment::CommitmentError},
    operation::{OperationBuilder, ProvenOperation},
};

pub use crate::abis::{Call, Shield};

mod abis;

/// `TintAdapter.relay`'s selector.
//? Hardcoded since `sol!` can't name `tint`'s `IPrivacyPool.Operation` type.
const RELAY_SELECTOR: [u8; 4] = [0x62, 0x72, 0x4a, 0x89];

/// Extension trait for [`OperationBuilder`] that relays through a [`TintAdapter`].
pub trait OperationBuilderAdapterExt {
    /// Relays this operation through `relay`'s adapter.
    ///
    /// Unshields `relay.inputs` to the adapter and binds the operation to the
    /// adapter and to `relay`'s calls and shields.
    #[must_use]
    fn with_relay(self, relay: &Relay) -> Self;
}

/// A deployed `TintAdapter` instance.
#[derive(Clone, Copy, Debug)]
pub struct TintAdapter {
    pub address: Address,
}

/// Dapp interactions a [`TintAdapter`] makes with an operation's unshielded funds.
#[derive(Clone, Debug)]
pub struct Relay {
    pub adapter: TintAdapter,
    /// Assets the operation unshields to the adapter for `calls` to spend.
    pub inputs: Vec<(AssetId, u128)>,
    /// Calls the adapter makes once the operation executes.
    pub calls: Vec<Call>,
    /// Assets the adapter shields back after `calls`.
    pub shields: Vec<Shield>,
}

#[derive(Debug, thiserror::Error)]
pub enum TintAdapterError {
    #[error("Commitment error: {0}")]
    Commitment(#[from] CommitmentError),
    #[error("Operation isn't bound to this relay")]
    RelayMismatch,
}

impl TintAdapter {
    #[must_use]
    pub fn relay(
        self,
        inputs: Vec<(AssetId, u128)>,
        calls: Vec<Call>,
        shields: Vec<Shield>,
    ) -> Relay {
        Relay {
            adapter: self,
            inputs,
            calls,
            shields,
        }
    }
}

impl Relay {
    /// A transaction relaying `operation`, built with [`OperationBuilderAdapterExt::with_relay`].
    ///
    /// # Errors
    /// Returns an error if `operation` isn't bound to this relay.
    pub fn transaction(
        &self,
        operation: &ProvenOperation,
    ) -> Result<TransactionRequest, TintAdapterError> {
        let operation = operation.operation();
        let context = &operation.context;
        if context.caller != self.adapter.address || context.binding != self.binding() {
            return Err(TintAdapterError::RelayMismatch);
        }

        let params = (operation.clone(), self.calls.clone(), self.shields.clone());
        let input = [RELAY_SELECTOR.as_slice(), &params.abi_encode_params()].concat();
        Ok(TransactionRequest::default()
            .to(self.adapter.address)
            .input(TransactionInput::new(input.into())))
    }

    /// The binding a `TintAdapter` operation for this relay must carry.
    fn binding(&self) -> B256 {
        keccak256((self.calls.clone(), self.shields.clone()).abi_encode_params())
    }
}

impl Call {
    #[must_use]
    pub fn new(target: Address, data: impl Into<Bytes>) -> Self {
        Self {
            target,
            value: U256::ZERO,
            data: data.into(),
        }
    }
}

impl Shield {
    /// Shields the adapter's balance of `asset` as a note spendable by `receiver`.
    ///
    /// # Errors
    /// Returns an error if the note can't be encrypted to `receiver`.
    pub fn new<R: RngCore + CryptoRng>(
        asset: Address,
        receiver: &PubAccount,
        rng: &mut R,
    ) -> Result<Self, TintAdapterError> {
        let mut random = B256::ZERO;
        rng.fill_bytes(random.as_mut_slice());

        //? The partial commitment doesn't cover the asset or amount.
        let commitment = receiver.commitment(AssetId::default(), 0, random);
        let encrypted = commitment
            .partial()
            .encrypt(&receiver.encryption_pub_key, rng)?;

        Ok(Self {
            asset,
            partialCommitment: fr_to_b256(commitment.partial_hash()),
            encryptedPartial: encrypted.into(),
        })
    }
}

impl OperationBuilderAdapterExt for OperationBuilder {
    fn with_relay(self, relay: &Relay) -> Self {
        let address = relay.adapter.address;
        relay
            .inputs
            .iter()
            .fold(self, |op, &(asset, amount)| {
                op.withdraw(address, asset, amount)
            })
            .caller(address)
            .binding(relay.binding())
    }
}

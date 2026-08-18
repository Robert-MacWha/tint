use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_sol_types::SolCall;
use ark_bn254::Fr;

use crate::{abis::tint::Tint, fr::fr_to_b256};

#[async_trait::async_trait]
pub trait Verifier {
    async fn verify(
        &self,
        index: u128,
        root: Fr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}

/// A [`Verifier`] that checks a specific Merkle root is registered on-chain
/// (via `RootRegistry.roots`) by a given block, confirming the indexer's
/// local tree state is genuinely anchored to what the contract committed.
pub struct RpcVerifier<P: Provider> {
    provider: P,
    contract: Address,
}

impl<P: Provider> RpcVerifier<P> {
    pub fn new(provider: P, contract: Address) -> Self {
        Self { provider, contract }
    }
}

#[async_trait::async_trait]
impl<P: Provider> Verifier for RpcVerifier<P> {
    async fn verify(
        &self,
        index: u128,
        root: Fr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let root = fr_to_b256(root);

        let call = Tint::getRootCall { index };

        let tx = TransactionRequest::default()
            .to(self.contract)
            .input(call.abi_encode().into());

        let result = self.provider.call(tx).await?;
        let expected_root = Tint::getRootCall::abi_decode_returns(&result)?;

        if root != expected_root {
            return Err(format!("root {root} not registered on-chain by index {index}").into());
        }
        Ok(())
    }
}

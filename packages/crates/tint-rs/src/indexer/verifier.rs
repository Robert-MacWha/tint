use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types_eth::{BlockNumberOrTag, TransactionRequest};
use alloy_sol_types::SolCall;
use ark_bn254::Fr;

use crate::{abis::tint::Tint, indexer::fr_to_b256};

#[async_trait::async_trait]
pub trait Verifier {
    async fn verify(
        &self,
        to: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}

/// A [`Verifier`] that checks a specific Merkle root is registered on-chain
/// (via `RootRegistry.roots`) by a given block, confirming the indexer's
/// local tree state is genuinely anchored to what the contract committed.
pub struct RootVerifier<P: Provider> {
    provider: P,
    contract: Address,
    root: Fr,
}

impl<P: Provider> RootVerifier<P> {
    pub fn new(provider: P, contract: Address, root: Fr) -> Self {
        Self {
            provider,
            contract,
            root,
        }
    }

}

#[async_trait::async_trait]
impl<P: Provider> Verifier for RootVerifier<P> {
    async fn verify(
        &self,
        to: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let call = Tint::rootsCall {
            root: fr_to_b256(self.root),
        };
        let tx = TransactionRequest::default()
            .to(self.contract)
            .input(call.abi_encode().into());

        let result = self
            .provider
            .call(tx)
            .block(BlockNumberOrTag::Number(to).into())
            .await?;
        let index = Tint::rootsCall::abi_decode_returns(&result)?;

        if index == 0 {
            return Err(format!(
                "root {} not registered on-chain by block {to}",
                fr_to_b256(self.root)
            )
            .into());
        }
        Ok(())
    }
}

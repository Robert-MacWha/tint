use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_sol_types::SolCall;
use ark_bn254::Fr;

use crate::{abis::tint::Tint, circuit::join_split::MMR_MAX_DEPTH, fr::fr_to_b256};

#[async_trait::async_trait]
pub trait Verifier {
    async fn verify(
        &self,
        hist_state: skew_mmr::state::State<MMR_MAX_DEPTH>,
        frontier: &[Option<Fr>],
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
        hist_state: skew_mmr::state::State<MMR_MAX_DEPTH>,
        frontier: &[Option<Fr>],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let hist_state = U256::from_le_bytes(*hist_state.bytes());
        let frontier = frontier.iter().flatten().copied().map(fr_to_b256).collect();

        let call = Tint::verifyFrontierCall {
            histState: hist_state,
            _frontier: frontier,
        };

        let tx = TransactionRequest::default()
            .to(self.contract)
            .input(call.abi_encode().into());

        let result = self.provider.call(tx).await?;
        let ok = Tint::verifyFrontierCall::abi_decode_returns(&result)?;

        if !ok {
            return Err("frontier verification failed".into());
        }

        Ok(())
    }
}

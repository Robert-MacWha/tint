use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_sol_types::SolCall;
use ark_bn254::Fr;

use crate::{abis::tint::Tint, fr::fr_to_b256, skew_mmr::SkewMmr};

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Tint accessors for any alloy [`Provider`].
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait TintProviderExt: Provider {
    /// Returns whether `nullifier` has been spent (`NullifierRegistry.isSpent`).
    async fn tint_is_spent(&self, tint: Address, nullifier: Fr) -> Result<bool, BoxError>;

    /// Returns whether `mmr`'s frontier and state match a frontier the
    /// on-chain skew-MMR held recently (`SkewMmrWithHistory.verifyFrontier`).
    async fn tint_verify_mmr(&self, tint: Address, mmr: &SkewMmr) -> Result<bool, BoxError>;
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl<P: Provider> TintProviderExt for P {
    async fn tint_is_spent(&self, tint: Address, nullifier: Fr) -> Result<bool, BoxError> {
        let call = Tint::isSpentCall {
            nullifierHash: fr_to_b256(nullifier),
        };
        call_view(self, tint, call).await
    }

    async fn tint_verify_mmr(&self, tint: Address, mmr: &SkewMmr) -> Result<bool, BoxError> {
        let state = mmr.state();
        let call = Tint::verifyFrontierCall {
            histRanks: U256::from_le_bytes(*state.packed_ranks()),
            histDepth: U256::from(state.depth()),
            histCount: U256::from(state.count()),
            frontier: mmr.roots().into_iter().flatten().map(fr_to_b256).collect(),
        };
        call_view(self, tint, call).await
    }
}

async fn call_view<P: Provider + ?Sized, C: SolCall>(
    provider: &P,
    to: Address,
    call: C,
) -> Result<C::Return, BoxError> {
    let tx = TransactionRequest::default()
        .to(to)
        .input(call.abi_encode().into());
    let result = provider.call(tx).await?;
    Ok(C::abi_decode_returns(&result)?)
}

//! Extension trait for paying for ERC-4337 userops with a Tint operation.
//!
//! The `TintPaymaster` contract executes a Tint operation during validation.
//! The operation unshields the gas fee in WETH to the paymaster. Its `caller`
//! is the paymaster, so only the paymaster can execute it, and its `binding`
//! commits to the userop's sender, so the proof can't be reused in another
//! userop. Anything else the operation unshields is available during
//! the userop's execution.

use alloy_primitives::{Address, B256, Bytes};
use alloy_rpc_types_eth::state::{StateOverride, StateOverridesBuilder};
use alloy_sol_types::SolValue;
use ark_bn254::Bn254;
use kohaku_userop_kit::{
    builder::UserOperationBuilder,
    bundler::{Bundler, BundlerError, BundlerExt},
};
use rand_core::{CryptoRng, RngCore};
use tint::{
    abis::tint::SIMULATION_SLOT,
    note::asset::AssetId,
    operation::{OperationBuilder, OperationError, ProvenOperation},
    policy::Authorizer,
    skew_mmr::SkewMmr,
};
use tint_groth16::artifacts::Artifacts;

/// Safety margin added to the quoted fee, in basis points.
const FEE_BUFFER_BPS: u128 = 100;

/// A deployed `TintPaymaster`.
#[derive(Clone, Copy, Debug)]
pub struct TintPaymaster {
    pub address: Address,
    /// The Tint pool the paymaster operates on.
    pub tint: Address,
    /// The paymaster's fee token.
    pub weth: Address,
}

#[derive(Debug, thiserror::Error)]
pub enum TintPaymasterError {
    #[error("Operation's gas exceeds the max fee: {required} > {max_fee}")]
    MaxFeeExceeded { required: u128, max_fee: u128 },
    #[error("Operation outgrew the proven fee: {required} > {fee}")]
    InsufficientFee { required: u128, fee: u128 },
    #[error("Authorization error: {0}")]
    Authorize(Box<dyn std::error::Error + Send + Sync>),
    #[error("Bundler error: {0}")]
    Bundler(#[from] BundlerError),
    #[error("Operation error: {0}")]
    Operation(#[from] OperationError),
}

/// Extension trait for [`UserOperationBuilder`] that pays for a userop's gas
/// with a Tint operation.
pub trait UserOperationPaymasterExt: Sized {
    /// Pays for this userop's gas out of `operation`, paying at most `max_fee`
    /// WETH.
    ///
    /// The paymaster's fee is added to `operation` as a WETH withdrawal, and
    /// the operation is bound to the userop's sender. `operation` must
    /// balance once the fee is withdrawn, e.g. by setting a change receiver.
    ///
    /// # Errors
    /// Returns an error if the quoted fee exceeds `max_fee`, or if
    /// authorization, proving or gas estimation fails.
    #[expect(clippy::too_many_arguments)]
    fn with_tint_paymaster<P, R>(
        self,
        paymaster: &TintPaymaster,
        operation: OperationBuilder,
        authorizer: &P,
        mmr: &SkewMmr,
        artifacts: &Artifacts<Bn254>,
        max_fee: u128,
        bundler: &dyn Bundler,
        rng: &mut R,
    ) -> impl Future<Output = Result<Self, TintPaymasterError>>
    where
        P: Authorizer,
        P::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        R: RngCore + CryptoRng;
}

impl<S> UserOperationPaymasterExt for UserOperationBuilder<S> {
    #[tracing::instrument(skip_all)]
    async fn with_tint_paymaster<P, R>(
        self,
        paymaster: &TintPaymaster,
        operation: OperationBuilder,
        authorizer: &P,
        mmr: &SkewMmr,
        artifacts: &Artifacts<Bn254>,
        max_fee: u128,
        bundler: &dyn Bundler,
        rng: &mut R,
    ) -> Result<Self, TintPaymasterError>
    where
        P: Authorizer,
        P::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        R: RngCore + CryptoRng,
    {
        let sender = self.build().user_op.sender;
        let operation = operation
            .caller(paymaster.address)
            .binding(sender.into_word());

        //? The paymaster executes the operation before the userop's calls, so
        //? the estimate needs one. Simulating it skips authorization.
        let simulated = simulate(paymaster, operation.clone(), authorizer, mmr, max_fee, rng)?;
        let builder = self
            .with_paymaster_and_data(paymaster.address, encode_paymaster_data(&simulated))
            .with_gas_estimate(bundler, Some(&simulation_override(paymaster.tint)))
            .await?;

        let cost = gas_cost(&builder);
        let fee = cost + cost * FEE_BUFFER_BPS / 10_000;
        if fee > max_fee {
            return Err(TintPaymasterError::MaxFeeExceeded {
                required: fee,
                max_fee,
            });
        }

        let proven = prove(paymaster, operation, authorizer, mmr, artifacts, fee, rng)?;
        let builder = builder
            .with_paymaster_and_data(paymaster.address, encode_paymaster_data(&proven))
            .with_gas_estimate(bundler, None)
            .await?;

        //? The proof is bound to `fee`, so a grown estimate can only be refused.
        let settled = gas_cost(&builder);
        if settled > fee {
            return Err(TintPaymasterError::InsufficientFee {
                required: settled,
                fee,
            });
        }

        Ok(builder)
    }
}

/// Assembles `operation` paying `fee`, with placeholder policy inputs and
/// proof. It only passes Tint under [`simulation_override`].
fn simulate<P: Authorizer, R: RngCore + CryptoRng>(
    paymaster: &TintPaymaster,
    operation: OperationBuilder,
    authorizer: &P,
    mmr: &SkewMmr,
    fee: u128,
    rng: &mut R,
) -> Result<ProvenOperation, TintPaymasterError> {
    let unauthorized = with_fee(operation, paymaster, fee).build(rng)?;
    let placeholder = authorizer.placeholder(&unauthorized);
    Ok(unauthorized
        .authorize(placeholder)?
        .simulate_with_mmr(mmr)?)
}

/// Authorizes and proves `operation` paying `fee`.
fn prove<P, R>(
    paymaster: &TintPaymaster,
    operation: OperationBuilder,
    authorizer: &P,
    mmr: &SkewMmr,
    artifacts: &Artifacts<Bn254>,
    fee: u128,
    rng: &mut R,
) -> Result<ProvenOperation, TintPaymasterError>
where
    P: Authorizer,
    P::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    R: RngCore + CryptoRng,
{
    let unauthorized = with_fee(operation, paymaster, fee).build(rng)?;
    let policy_inputs = authorizer
        .authorize(&unauthorized)
        .map_err(|e| TintPaymasterError::Authorize(e.into()))?;
    Ok(unauthorized
        .authorize(policy_inputs)?
        .prove_with_mmr(mmr, artifacts, rng)?)
}

/// Adds the paymaster's `fee` to `operation` as a WETH withdrawal.
fn with_fee(operation: OperationBuilder, paymaster: &TintPaymaster, fee: u128) -> OperationBuilder {
    operation.withdraw(paymaster.address, AssetId::from(paymaster.weth), fee)
}

/// Sets Tint's simulation flag, so a simulated operation passes.
fn simulation_override(tint: Address) -> StateOverride {
    StateOverridesBuilder::default()
        .with_state_diff(tint, [(SIMULATION_SLOT, B256::with_last_byte(1))])
        .build()
}

/// The userop's maximum gas cost in wei, which WETH pays 1:1.
fn gas_cost<S>(builder: &UserOperationBuilder<S>) -> u128 {
    let user_op = builder.build();
    let price = user_op.user_op.max_fee_per_gas + user_op.user_op.max_priority_fee_per_gas;
    user_op.total_gas_limit() * price
}

fn encode_paymaster_data(proven: &ProvenOperation) -> Bytes {
    proven.operation().abi_encode().into()
}

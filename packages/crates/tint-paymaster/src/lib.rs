//! Extension trait for paying for ERC-4337 userops with a Tint operation.

use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use kohaku_userop_kit::{
    builder::UserOperationBuilder,
    bundler::{Bundler, BundlerError, BundlerExt},
};
use rand_core::{CryptoRng, RngCore};
use tint::{
    note::asset::AssetId,
    operation::{Operation, OperationBuilder, OperationError, ProvenOperation},
};

/// A deployed `TintPaymaster`.
#[derive(Clone, Copy, Debug)]
pub struct TintPaymaster {
    pub address: Address,
    /// The paymaster's fee token.
    pub weth: Address,
}

#[derive(Debug, thiserror::Error)]
pub enum TintPaymasterError {
    #[error("Bundler error: {0}")]
    Bundler(#[from] BundlerError),
    #[error("Operation error: {0}")]
    Operation(#[from] OperationError),
}

/// Extension trait for [`UserOperationBuilder`] that pays with a [`TintPaymaster`].
pub trait UserOperationPaymasterExt: Sized {
    /// Builds `operation` to pay `fee` WETH for this userop's gas, ready to
    /// authorize and prove.
    ///
    /// Adds the fee as a WETH withdrawal to the paymaster, and binds the
    /// operation to the paymaster and this userop's sender. `operation` must
    /// balance once the fee is withdrawn, e.g. by setting a change receiver.
    ///
    /// # Errors
    /// Returns an error if the operation doesn't build.
    fn tint_paymaster_operation<R: RngCore + CryptoRng>(
        &self,
        paymaster: &TintPaymaster,
        operation: OperationBuilder,
        fee: u128,
        rng: &mut R,
    ) -> Result<Operation, TintPaymasterError>;

    /// Pays for this userop's gas with `operation`, built by
    /// [`Self::tint_paymaster_operation`].
    ///
    /// The paymaster rejects the userop if its fee doesn't cover the userop's
    /// maximum gas cost.
    ///
    /// # Errors
    /// Returns an error if gas estimation fails.
    fn with_tint_paymaster(
        self,
        paymaster: &TintPaymaster,
        operation: &ProvenOperation,
        bundler: &dyn Bundler,
    ) -> impl Future<Output = Result<Self, TintPaymasterError>>;
}

impl<S> UserOperationPaymasterExt for UserOperationBuilder<S> {
    fn tint_paymaster_operation<R: RngCore + CryptoRng>(
        &self,
        paymaster: &TintPaymaster,
        operation: OperationBuilder,
        fee: u128,
        rng: &mut R,
    ) -> Result<Operation, TintPaymasterError> {
        let sender = self.build().user_op.sender;
        Ok(operation
            .withdraw(paymaster.address, AssetId::from(paymaster.weth), fee)
            .caller(paymaster.address)
            .binding(sender.into_word())
            .build(rng)?)
    }

    #[tracing::instrument(skip_all)]
    async fn with_tint_paymaster(
        self,
        paymaster: &TintPaymaster,
        operation: &ProvenOperation,
        bundler: &dyn Bundler,
    ) -> Result<Self, TintPaymasterError> {
        let data = operation.operation().abi_encode();
        Ok(self
            .with_paymaster_and_data(paymaster.address, data.into())
            .with_gas_estimate(bundler, None)
            .await?)
    }
}

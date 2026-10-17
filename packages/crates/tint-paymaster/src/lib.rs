//! Extension trait for paying for ERC-4337 userops with a Tint operation.

use alloy_primitives::{Address, B256, Bytes, keccak256};
use alloy_sol_types::SolValue;
use kohaku_userop_kit::{
    builder::UserOperationBuilder,
    bundler::{Bundler, BundlerError, BundlerExt},
};
use rand_core::{CryptoRng, RngCore};
use tint::{
    account::pub_account::PubAccount,
    fr::fr_to_b256,
    note::{asset::AssetId, commitment::CommitmentError},
    operation::{Operation, OperationBuilder, OperationError, ProvenOperation},
};

mod abis;

/// A deployed `TintPaymaster`.
#[derive(Clone, Copy, Debug)]
pub struct TintPaymaster {
    pub address: Address,
    /// The paymaster's fee token.
    pub weth: Address,
}

/// Where the paymaster refunds the fee left over after the userop's gas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refund {
    /// The paymaster keeps the whole fee.
    None,
    /// Transfers the refund in WETH to an address.
    Transfer(Address),
    /// Deposits the refund into Tint as a WETH note. Build with [`Refund::deposit`].
    Deposit {
        partial_commitment: B256,
        encrypted_partial: Bytes,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum TintPaymasterError {
    #[error("Bundler error: {0}")]
    Bundler(#[from] BundlerError),
    #[error("Operation error: {0}")]
    Operation(#[from] OperationError),
    #[error("Commitment error: {0}")]
    Commitment(#[from] CommitmentError),
    #[error("Operation isn't bound to this refund and userop sender")]
    RefundMismatch,
}

/// Extension trait for [`UserOperationBuilder`] that pays with a [`TintPaymaster`].
pub trait UserOperationPaymasterExt: Sized {
    /// Builds `operation` to pay `fee` WETH for this userop's gas, ready to
    /// authorize and prove.
    ///
    /// Adds the fee as a WETH withdrawal to the paymaster, and binds the
    /// operation to the paymaster, this userop's sender and `refund`.
    /// `operation` must balance once the fee is withdrawn, e.g. by setting a
    /// change receiver.
    ///
    /// # Errors
    /// Returns an error if the operation doesn't build.
    fn tint_paymaster_operation<R: RngCore + CryptoRng>(
        &self,
        paymaster: &TintPaymaster,
        operation: OperationBuilder,
        fee: u128,
        refund: &Refund,
        rng: &mut R,
    ) -> Result<Operation, TintPaymasterError>;

    /// Pays for this userop's gas with `operation`, built by
    /// [`Self::tint_paymaster_operation`] with the same `refund`.
    ///
    /// The paymaster rejects the userop if its fee doesn't cover the userop's
    /// maximum gas cost.
    ///
    /// # Errors
    /// Returns an error if `operation` isn't bound to `refund`, or if gas
    /// estimation fails.
    fn with_tint_paymaster(
        self,
        paymaster: &TintPaymaster,
        operation: &ProvenOperation,
        refund: &Refund,
        bundler: &dyn Bundler,
    ) -> impl Future<Output = Result<Self, TintPaymasterError>>;
}

impl<S> UserOperationPaymasterExt for UserOperationBuilder<S> {
    fn tint_paymaster_operation<R: RngCore + CryptoRng>(
        &self,
        paymaster: &TintPaymaster,
        operation: OperationBuilder,
        fee: u128,
        refund: &Refund,
        rng: &mut R,
    ) -> Result<Operation, TintPaymasterError> {
        let sender = self.build().user_op.sender;
        Ok(operation
            .withdraw(paymaster.address, AssetId::from(paymaster.weth), fee)
            .caller(paymaster.address)
            .binding(refund.binding(sender))
            .build(rng)?)
    }

    #[tracing::instrument(skip_all)]
    async fn with_tint_paymaster(
        self,
        paymaster: &TintPaymaster,
        operation: &ProvenOperation,
        refund: &Refund,
        bundler: &dyn Bundler,
    ) -> Result<Self, TintPaymasterError> {
        let sender = self.build().user_op.sender;
        let operation = operation.operation();
        if operation.context.binding != refund.binding(sender) {
            return Err(TintPaymasterError::RefundMismatch);
        }

        let data = (operation.clone(), refund.to_sol()).abi_encode_params();
        Ok(self
            .with_paymaster_and_data(paymaster.address, data.into())
            .with_gas_estimate(bundler, None)
            .await?)
    }
}

impl Refund {
    pub fn transfer(recipient: Address) -> Self {
        Self::Transfer(recipient)
    }

    /// A refund deposited into Tint as a WETH note spendable by `receiver`.
    ///
    /// # Errors
    /// Returns an error if the note can't be encrypted to `receiver`.
    pub fn deposit<R: RngCore + CryptoRng>(
        receiver: &PubAccount,
        rng: &mut R,
    ) -> Result<Self, TintPaymasterError> {
        let mut random = B256::ZERO;
        rng.fill_bytes(random.as_mut_slice());

        let commitment = receiver.commitment(AssetId::default(), 0, random);
        let encrypted = commitment
            .partial()
            .encrypt(&[receiver.encryption_pub_key], rng)?;

        Ok(Self::Deposit {
            partial_commitment: fr_to_b256(commitment.partial_hash()),
            encrypted_partial: encrypted.into(),
        })
    }

    /// The binding a `TintPaymaster` operation for `sender` must carry.
    fn binding(&self, sender: Address) -> B256 {
        keccak256((sender, self.to_sol()).abi_encode_params())
    }

    fn to_sol(&self) -> abis::Refund {
        self.clone().into()
    }
}

impl From<Refund> for abis::Refund {
    fn from(refund: Refund) -> Self {
        match refund {
            Refund::None => abis::Refund {
                kind: abis::RefundKind::None,
                recipient: Address::ZERO,
                partialCommitment: B256::ZERO,
                encryptedPartial: Bytes::new(),
            },
            Refund::Transfer(recipient) => abis::Refund {
                kind: abis::RefundKind::Transfer,
                recipient: recipient,
                partialCommitment: B256::ZERO,
                encryptedPartial: Bytes::new(),
            },
            Refund::Deposit {
                partial_commitment,
                encrypted_partial,
            } => abis::Refund {
                kind: abis::RefundKind::Deposit,
                recipient: Address::ZERO,
                partialCommitment: partial_commitment,
                encryptedPartial: encrypted_partial.clone(),
            },
        }
    }
}

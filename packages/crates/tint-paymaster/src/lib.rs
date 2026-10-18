//! Extension traits for paying for ERC-4337 userops with a Tint operation.

use alloy_primitives::{Address, B256, Bytes, keccak256};
use alloy_rpc_types_eth::state::{StateOverride, StateOverridesBuilder};
use alloy_sol_types::SolValue;
use kohaku_userop_kit::builder::UserOperationBuilder;
use rand_core::{CryptoRng, RngCore};
use tint::{
    abis::tint::SIMULATION_SLOT,
    account::pub_account::PubAccount,
    fr::fr_to_b256,
    note::{asset::AssetId, commitment::CommitmentError},
    operation::{OperationBuilder, ProvenOperation},
};

mod abis;

/// A deployed `TintPaymaster` instance.
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

/// `sender`'s userop payment through a [`TintPaymaster`].
#[derive(Clone, Debug)]
pub struct TintPayment {
    pub paymaster: TintPaymaster,
    pub fee: u128,
    pub refund: Refund,
    pub sender: Address,
}

#[derive(Debug, thiserror::Error)]
pub enum TintPaymasterError {
    #[error("Commitment error: {0}")]
    Commitment(#[from] CommitmentError),
    #[error("Operation isn't bound to this refund and userop sender")]
    RefundMismatch,
}

/// Extension trait for [`OperationBuilder`] that pays a userop's gas through a
/// [`TintPaymaster`].
pub trait OperationBuilderPaymasterExt {
    /// Use `payment` to sponsor this operation.
    ///
    /// Adds a withdrawal of `payment.fee` WETH to the paymaster amd
    /// binds the operation to the given refund and sender.
    #[must_use]
    fn with_paymaster(self, payment: &TintPayment) -> Self;
}

/// Extension trait for [`UserOperationBuilder`] that pays with a [`TintPaymaster`].
pub trait UserOperationPaymasterExt: Sized {
    /// Pays for this userop's gas with `operation`.
    ///
    /// Built with [`OperationBuilderPaymasterExt::with_paymaster`] and the
    /// same `payment`.
    ///
    /// The userop will be rejected if its fee doesn't cover the userop's
    /// maximum gas cost.
    ///
    /// # Errors
    /// Returns an error if this userop's sender isn't `payment.sender`, or
    /// `operation` isn't bound to `payment`.
    fn with_tint_paymaster(
        self,
        payment: &TintPayment,
        operation: &ProvenOperation,
    ) -> Result<Self, TintPaymasterError>;

    /// The estimated fee the paymaster requires for this userop.
    fn max_gas_cost(&self) -> u128;
}

impl OperationBuilderPaymasterExt for OperationBuilder {
    fn with_paymaster(self, payment: &TintPayment) -> Self {
        let TintPaymaster { address, weth } = payment.paymaster;
        self.withdraw(address, AssetId::from(weth), payment.fee)
            .caller(address)
            .binding(payment.refund.binding(payment.sender))
    }
}

impl<S> UserOperationPaymasterExt for UserOperationBuilder<S> {
    fn with_tint_paymaster(
        self,
        payment: &TintPayment,
        operation: &ProvenOperation,
    ) -> Result<Self, TintPaymasterError> {
        let sender = self.build().user_op.sender;
        let operation = operation.operation();
        let binding = payment.refund.binding(payment.sender);
        if sender != payment.sender || operation.context.binding != binding {
            return Err(TintPaymasterError::RefundMismatch);
        }

        let data = (operation.clone(), payment.refund.to_sol()).abi_encode_params();
        Ok(self.with_paymaster_and_data(payment.paymaster.address, data.into()))
    }

    fn max_gas_cost(&self) -> u128 {
        let op = self.build();
        op.total_gas_limit() * op.user_op.max_fee_per_gas
    }
}

impl TintPaymaster {
    #[must_use]
    pub fn pay(self, fee: u128, refund: Refund, sender: Address) -> TintPayment {
        TintPayment {
            paymaster: self,
            fee,
            refund,
            sender,
        }
    }

    /// A state override for simulating the tint-sponsored userop.
    ///
    /// Allows simulating the operation without needing valid proofs for tint
    /// or its policies by enabling tint's simulation flag.
    #[must_use]
    pub fn simulation_override(&self, tint: Address) -> StateOverride {
        let flag = [(SIMULATION_SLOT, B256::with_last_byte(1))];
        StateOverridesBuilder::default()
            .with_state_diff(tint, flag)
            .with_state_diff(self.address, flag)
            .build()
    }
}

impl Refund {
    #[must_use]
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
                recipient,
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

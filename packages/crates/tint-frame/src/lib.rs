//! Tint operations that pay their own gas from shielded WETH, sent as EIP-8141 frame
//! transactions whose sender is a `TintFrame` deployment:
//!
//! | # | mode    | target     | data                   |
//! |---|---------|------------|------------------------|
//! | 0 | VERIFY  | `TintFrame` | `validate()`           |
//! | 1 | SENDER  | `TintFrame` | `executeSponsored(op)` |
//! | 2.. | DEFAULT | any      | the caller's calls     |
//!
//! The operation unshields a WETH fee covering the transaction's `max_cost` to `TintFrame`, and
//! binds itself to a one-time authorizer key, whose signature covers the whole transaction, by
//! setting its `binding` to the key's address.
//!
//! ```ignore
//! let sponsored = SponsoredOperation::new(tint_frame, weth, chain_id, max_fee, max_priority_fee)
//!     .call(router, swap_calldata, 200_000);
//! let proven = sponsored
//!     .prepare(OperationBuilder::new().spend(note).withdraw(router, asset, amount))?
//!     .change_to(me)
//!     .build(&mut rng)?
//!     .authorize(policy_inputs)?
//!     .prove_with_mmr(&mmr, &artifacts, &mut rng)?;
//! provider.send_frame_transaction(&sponsored.transaction(&proven)?).await?;
//! ```

use alloy_hegota::{ApproveScope, Frame, FrameMode, FrameSignature, TxFrame, tx::SignError};
use alloy_primitives::{Address, Bytes, U256};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use tint::{
    abis::tint::TintFrame,
    fr::fr_to_u256,
    note::asset::AssetId,
    operation::{OperationBuilder, ProvenOperation},
};

/// Frame 1's execution gas. Must match `TintFrame.MIN_EXECUTE_GAS`.
pub const EXECUTE_GAS: u64 = 1_000_000;
/// Frame 1's state gas. Must match `TintFrame.MIN_EXECUTE_STATE_GAS`.
pub const EXECUTE_STATE_GAS: u64 = 2_500_000;
/// Frame 0's execution gas: `verifyOperation` plus the frame checks, within the devnet's 500k
/// validation budget.
pub const VERIFY_GAS: u64 = 450_000;
/// Frame 0's state gas: consuming one new nonce key (64 bytes at 1,530 gas per byte).
pub const VERIFY_STATE_GAS: u64 = 97_920;
/// The longest `executeSponsored(op)` calldata the fee is sized for.
pub const MAX_OPERATION_BYTES: usize = 8192;

#[derive(Debug, thiserror::Error)]
pub enum SponsorError {
    #[error("the fee, {0} wei, doesn't fit in a u128")]
    FeeTooLarge(U256),
    #[error("executeSponsored calldata is {0} bytes, over MAX_OPERATION_BYTES")]
    OperationTooLarge(usize),
    #[error("the operation spends no notes")]
    NoInputs,
    #[error("the operation spends a note with a policy, which TintFrame doesn't sponsor")]
    PolicyNotSupported,
    #[error(transparent)]
    Sign(#[from] SignError),
}

/// A sponsored transaction's shape: the `TintFrame` paying for it, its fees, and the calls
/// that follow the operation.
pub struct SponsoredOperation {
    tint_frame: Address,
    weth: Address,
    chain_id: u64,
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
    calls: Vec<Frame>,
    authorizer: PrivateKeySigner,
}

impl SponsoredOperation {
    #[must_use]
    pub fn new(
        tint_frame: Address,
        weth: Address,
        chain_id: u64,
        max_fee_per_gas: U256,
        max_priority_fee_per_gas: U256,
    ) -> Self {
        Self {
            tint_frame,
            weth,
            chain_id,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            calls: Vec::new(),
            authorizer: PrivateKeySigner::random(),
        }
    }

    /// Appends a call to `target`, made by the EIP-8141 entry point after the operation.
    /// Add every call before [`Self::prepare`], since they change the fee.
    #[must_use]
    pub fn call(mut self, target: Address, data: Bytes, gas_limit: u64) -> Self {
        self.calls.push(Frame {
            mode: FrameMode::Default,
            target: Some(target),
            gas_limit,
            data,
            ..Frame::default()
        });
        self
    }

    /// The WETH fee: the transaction's `max_cost` with the longest operation it allows.
    /// Gas grows with calldata, so this covers any operation that fits.
    pub fn fee(&self) -> Result<u128, SponsorError> {
        let mut tx = self.unsigned(U256::MAX, vec![0xff; MAX_OPERATION_BYTES].into());
        tx.signatures[0] = FrameSignature {
            signer: Address::repeat_byte(0xff),
            signature: vec![0xff; 65].into(),
            ..tx.signatures[0].clone()
        };
        let fee = tx.max_cost(U256::ZERO);
        u128::try_from(fee).map_err(|_| SponsorError::FeeTooLarge(fee))
    }

    /// Adds the fee withdrawal and the authorizer binding to `builder`. This uses one of the
    /// operation's withdrawal slots.
    pub fn prepare(&self, builder: OperationBuilder) -> Result<OperationBuilder, SponsorError> {
        Ok(builder
            .withdraw(self.tint_frame, AssetId::from(self.weth), self.fee()?)
            .binding(self.authorizer.address().into_word()))
    }

    /// The signed frame transaction submitting `op`, which must have been built from
    /// [`Self::prepare`] and spend only notes without a policy.
    pub fn transaction(&self, op: &ProvenOperation) -> Result<TxFrame, SponsorError> {
        let nullifier = op.nullifiers().first().ok_or(SponsorError::NoInputs)?;
        let policy_inputs = &op.operation().context.policyInputs;
        if policy_inputs.iter().any(|input| !input.is_empty()) {
            return Err(SponsorError::PolicyNotSupported);
        }
        let execute = TintFrame::executeSponsoredCall::new((op.operation().clone(),)).abi_encode();
        if execute.len() > MAX_OPERATION_BYTES {
            return Err(SponsorError::OperationTooLarge(execute.len()));
        }

        let mut tx = self.unsigned(fr_to_u256(*nullifier), execute.into());
        tx.sign_secp256k1(0, &self.authorizer)?;
        Ok(tx)
    }

    /// The transaction before its authorizer signs it. Its nonce key is the operation's first
    /// nullifier: unique per operation, so concurrent operations don't share a nonce.
    fn unsigned(&self, nonce_key: U256, execute: Bytes) -> TxFrame {
        let validate = Frame {
            mode: FrameMode::Verify,
            approve_scope: ApproveScope::ExecutionAndPayment,
            target: Some(self.tint_frame),
            gas_limit: VERIFY_GAS,
            state_gas_limit: VERIFY_STATE_GAS,
            data: TintFrame::validateCall {}.abi_encode().into(),
            ..Frame::default()
        };
        let execute = Frame {
            mode: FrameMode::Sender,
            target: Some(self.tint_frame),
            gas_limit: EXECUTE_GAS,
            state_gas_limit: EXECUTE_STATE_GAS,
            data: execute,
            ..Frame::default()
        };

        TxFrame {
            chain_id: self.chain_id,
            nonce_keys: vec![nonce_key],
            nonce_seq: 0,
            sender: self.tint_frame,
            frames: [validate, execute]
                .into_iter()
                .chain(self.calls.clone())
                .collect(),
            signatures: vec![FrameSignature::default()],
            max_priority_fee_per_gas: self.max_priority_fee_per_gas,
            max_fee_per_gas: self.max_fee_per_gas,
            max_fee_per_blob_gas: U256::ZERO,
            blob_versioned_hashes: Vec::new(),
        }
    }
}

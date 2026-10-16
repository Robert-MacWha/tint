//! alloy extensions for EIP-8141 frame transactions as the hegota devnet (ethrex
//! `hegota-testnet`) runs them: the transaction model and its encoding, signing and gas
//! bounds ([`tx`]), receipts ([`receipt`]), and provider calls ([`provider`]).
//!
//! The encoding follows ethrex rather than the published EIP: EIP-8250 replaces the nonce
//! with `nonce_keys` and `nonce_seq`, and frames carry separate execution and state (EIP-8037)
//! gas limits.

pub mod provider;
pub mod receipt;
pub mod tx;

use alloy_primitives::{Address, address};
pub use provider::{ExecutionStatus, HegotaError, HegotaProviderExt, SimulatedFrame, Simulation};
pub use receipt::{FrameReceipt, FrameStatus, FrameTxReceipt, TxStatus};
pub use tx::{ApproveScope, Frame, FrameMode, FrameSignature, SignatureScheme, TxFrame};

/// The caller of DEFAULT and VERIFY frames.
pub const ENTRY_POINT: Address = address!("0x00000000000000000000000000000000000000aa");

/// The EIP-2718 type byte of a frame transaction.
pub const FRAME_TX_TYPE: u8 = 0x06;

//! The frame transaction model, mirroring ethrex and minimal-shielded-pool's `frametx.ts`:
//!
//! ```text
//! raw       = 0x06 || rlp([chain_id, nonce_keys, nonce_seq, sender, frames, signatures,
//!                          [max_priority_fee, max_fee, max_blob_fee], blob_hashes])
//! frame     = [mode, flags, target or empty, [gas_limit, state_gas_limit], value, data]
//! signature = [scheme, signer, msg, signature]
//! sig_hash  = keccak256(raw, with the bytes of every empty-msg signature elided)
//! ```

use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use alloy_rlp::{BufMut, Encodable, Header};
use alloy_signer::{Signer, SignerSync};

use crate::FRAME_TX_TYPE;

/// Who calls a frame's target, and what the frame may do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameMode {
    /// Called by the [`ENTRY_POINT`](crate::ENTRY_POINT).
    #[default]
    Default = 0,
    /// Validates in static mode, and may APPROVE. If it reverts, the transaction is invalid.
    Verify = 1,
    /// Called by the transaction's sender, once execution is approved.
    Sender = 2,
}

/// What APPROVE may grant in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ApproveScope {
    #[default]
    None = 0,
    Payment = 1,
    Execution = 2,
    ExecutionAndPayment = 3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum SignatureScheme {
    /// Raw bytes the protocol doesn't check, readable on-chain with SIGDATACOPY.
    #[default]
    Arbitrary = 0,
    Secp256k1 = 1,
    P256 = 2,
}

/// One call of a frame transaction.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frame {
    pub mode: FrameMode,
    pub approve_scope: ApproveScope,
    /// Joins this frame to the next one in an atomic batch.
    pub atomic_batch: bool,
    /// The called address; `None` targets the transaction's sender.
    pub target: Option<Address>,
    /// Execution gas limit.
    pub gas_limit: u64,
    /// EIP-8037 state gas limit, a separate budget execution can't borrow from.
    pub state_gas_limit: u64,
    /// Only SENDER frames may carry value.
    pub value: U256,
    pub data: Bytes,
}

/// A signature the protocol checks before any frame runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameSignature {
    pub scheme: SignatureScheme,
    pub signer: Address,
    /// The digest signed. Empty signs the transaction's [`TxFrame::sig_hash`].
    pub msg: Bytes,
    /// For secp256k1, `v (0 or 1) || r || s`.
    pub signature: Bytes,
}

/// An EIP-8141 frame transaction (type `0x06`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TxFrame {
    pub chain_id: u64,
    /// EIP-8250 nonce keys: strictly increasing; key 0 is the account nonce and must be alone.
    pub nonce_keys: Vec<U256>,
    pub nonce_seq: u64,
    pub sender: Address,
    pub frames: Vec<Frame>,
    pub signatures: Vec<FrameSignature>,
    pub max_priority_fee_per_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_fee_per_blob_gas: U256,
    pub blob_versioned_hashes: Vec<B256>,
}

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("no signature at index {0}")]
    NoSignature(usize),
    #[error("signature {0}'s msg is {1} bytes, not empty or a 32-byte digest")]
    InvalidMsg(usize, usize),
    #[error(transparent)]
    Signer(#[from] alloy_signer::Error),
}

const BLOB_GAS_PER_BLOB: u64 = 131_072;

impl TxFrame {
    /// The bytes `eth_sendRawTransaction` takes.
    #[must_use]
    pub fn encoded_2718(&self) -> Vec<u8> {
        self.encode_typed(false)
    }

    /// What every empty-msg signature signs.
    #[must_use]
    pub fn sig_hash(&self) -> B256 {
        keccak256(self.encode_typed(true))
    }

    /// Sets signature `index`'s scheme and signer, then signs its `msg` or, when that's
    /// empty, [`Self::sig_hash`] (which covers the scheme and signer).
    pub fn sign_secp256k1(
        &mut self,
        index: usize,
        signer: &(impl SignerSync + Signer),
    ) -> Result<(), SignError> {
        let sig = self
            .signatures
            .get_mut(index)
            .ok_or(SignError::NoSignature(index))?;
        sig.scheme = SignatureScheme::Secp256k1;
        sig.signer = signer.address();
        let digest = match sig.msg.len() {
            0 => self.sig_hash(),
            32 => B256::from_slice(&sig.msg),
            len => return Err(SignError::InvalidMsg(index, len)),
        };

        let signature = signer.sign_hash_sync(&digest)?;
        let mut bytes = Vec::with_capacity(65);
        bytes.push(u8::from(signature.v()));
        bytes.extend_from_slice(&signature.r().to_be_bytes::<32>());
        bytes.extend_from_slice(&signature.s().to_be_bytes::<32>());
        self.signatures[index].signature = bytes.into();
        Ok(())
    }

    /// The most the payer can be charged, at `blob_base_fee`. Saturates at `U256::MAX`.
    #[must_use]
    pub fn max_cost(&self, blob_base_fee: U256) -> U256 {
        let blob_gas = U256::from(self.blob_versioned_hashes.len() as u64 * BLOB_GAS_PER_BLOB);
        self.max_fee_per_gas
            .saturating_mul(U256::from(self.max_gas()))
            .saturating_add(blob_gas.saturating_mul(blob_base_fee))
    }

    /// EIP-8141 `max_gas`: the larger of the standard limit and the calldata floor, with
    /// state gas on top of either.
    #[must_use]
    pub fn max_gas(&self) -> u128 {
        let state: u128 = self
            .frames
            .iter()
            .map(|f| u128::from(f.state_gas_limit))
            .sum();
        let execution: u128 = self.frames.iter().map(|f| u128::from(f.gas_limit)).sum();
        let standard = self.mandatory_gas() + self.calldata_gas() + execution + state;
        let floor = self.mandatory_gas() + 64 * self.data_len() + state;
        standard.max(floor)
    }

    /// Intrinsic gas, per-frame gas, signature verification, and value transfers.
    #[must_use]
    pub fn mandatory_gas(&self) -> u128 {
        let frames = 475 * self.frames.len() as u128;
        let signatures: u128 = self
            .signatures
            .iter()
            .map(|s| s.scheme.verification_cost())
            .sum();
        let transfers = self
            .frames
            .iter()
            .filter(|f| !f.value.is_zero() && f.target.is_some_and(|t| t != self.sender))
            .count() as u128;
        12_000 + frames + signatures + 6_000 * transfers
    }

    /// 4 gas per zero byte and 16 per other byte of the [data fields](Self::data_fields).
    fn calldata_gas(&self) -> u128 {
        self.data_fields()
            .flatten()
            .map(|b| if b == 0 { 4 } else { 16 })
            .sum()
    }

    fn data_len(&self) -> u128 {
        self.data_fields().map(|f| f.len() as u128).sum()
    }

    /// What EIP-8141 prices as data: frame data, each signature's signer, msg and bytes,
    /// and the RLP of the nonce keys followed by the nonce sequence.
    fn data_fields(&self) -> impl Iterator<Item = Vec<u8>> + '_ {
        let mut nonces = Vec::new();
        alloy_rlp::encode_list::<_, U256>(&self.nonce_keys, &mut nonces);
        self.nonce_seq.encode(&mut nonces);

        let frames = self.frames.iter().map(|f| f.data.to_vec());
        let signatures = self
            .signatures
            .iter()
            .flat_map(|s| [s.signer.to_vec(), s.msg.to_vec(), s.signature.to_vec()]);
        frames.chain(signatures).chain(std::iter::once(nonces))
    }

    fn encode_typed(&self, elide_signatures: bool) -> Vec<u8> {
        let mut out = vec![FRAME_TX_TYPE];
        list_with(&mut out, |out| {
            self.chain_id.encode(out);
            alloy_rlp::encode_list::<_, U256>(&self.nonce_keys, out);
            self.nonce_seq.encode(out);
            self.sender.encode(out);
            list_with(out, |out| {
                for frame in &self.frames {
                    frame.encode(out);
                }
            });
            list_with(out, |out| {
                for sig in &self.signatures {
                    sig.encode(out, elide_signatures);
                }
            });
            list_with(out, |out| {
                self.max_priority_fee_per_gas.encode(out);
                self.max_fee_per_gas.encode(out);
                self.max_fee_per_blob_gas.encode(out);
            });
            alloy_rlp::encode_list::<_, B256>(&self.blob_versioned_hashes, out);
        });
        out
    }
}

impl SignatureScheme {
    fn verification_cost(self) -> u128 {
        match self {
            Self::Arbitrary => 100,
            Self::Secp256k1 => 2_800,
            Self::P256 => 6_700,
        }
    }
}

impl Frame {
    fn flags(&self) -> u8 {
        self.approve_scope as u8 | u8::from(self.atomic_batch) << 2
    }

    fn encode(&self, out: &mut dyn BufMut) {
        list_with(out, |out| {
            (self.mode as u8).encode(out);
            self.flags().encode(out);
            match self.target {
                Some(target) => target.encode(out),
                None => Bytes::new().encode(out),
            }
            list_with(out, |out| {
                self.gas_limit.encode(out);
                self.state_gas_limit.encode(out);
            });
            self.value.encode(out);
            self.data.encode(out);
        });
    }
}

impl FrameSignature {
    /// With `elide`, an empty-msg signature encodes no signature bytes: the hash it signs
    /// can't cover them.
    fn encode(&self, out: &mut dyn BufMut, elide: bool) {
        list_with(out, |out| {
            (self.scheme as u8).encode(out);
            self.signer.encode(out);
            self.msg.encode(out);
            if elide && self.msg.is_empty() {
                Bytes::new().encode(out);
            } else {
                self.signature.encode(out);
            }
        });
    }
}

/// Writes the RLP list whose already-encoded items `body` writes.
fn list_with(out: &mut dyn BufMut, body: impl FnOnce(&mut Vec<u8>)) {
    let mut payload = Vec::new();
    body(&mut payload);
    Header {
        list: true,
        payload_length: payload.len(),
    }
    .encode(out);
    out.put_slice(&payload);
}

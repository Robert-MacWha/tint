//! Checks the encoding, signature hash and gas bounds against minimal-shielded-pool's pinned
//! reference vectors (`test/vectors/reference/frametx.json`: its `transactions`, and its
//! `selftest` cases that are transactions).

// Unwraps in the JSON-parsing helpers below, which clippy doesn't count as tests.
#![allow(clippy::unwrap_used)]

use std::str::FromStr;

use alloy_hegota::{
    ApproveScope, Frame, FrameMode, FrameSignature, FrameStatus, FrameTxReceipt, SignatureScheme,
    TxFrame, TxStatus,
};
use alloy_primitives::{Address, B256, Bytes, Signature, U256, hex, keccak256};
use alloy_signer_local::PrivateKeySigner;
use serde_json::Value;

const VECTORS: &str = include_str!("fixtures/frametx.json");

#[test]
fn matches_reference_vectors() {
    let vectors: Value = serde_json::from_str(VECTORS).unwrap();
    let cases = [&vectors["transactions"], &vectors["selftest"]]
        .into_iter()
        .flat_map(|cases| cases.as_array().unwrap());

    let mut checked = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        // Random cases with invalid modes, flags, schemes or blob hashes don't parse.
        let Some(tx) = parse_tx(&case["tx"]) else {
            continue;
        };
        let out = &case["out"];
        checked += 1;

        // Long encodings are pinned by length and hash.
        let raw = tx.encoded_2718();
        if let Some(expected) = out["raw"].as_str() {
            assert_eq!(hex::encode_prefixed(&raw), expected, "{name}: raw");
        } else {
            assert_eq!(
                raw.len() as u128,
                int(&out["rawLength"]),
                "{name}: rawLength"
            );
            let expected = B256::from_str(out["rawKeccak"].as_str().unwrap()).unwrap();
            assert_eq!(keccak256(&raw), expected, "{name}: rawKeccak");
        }
        assert_eq!(
            tx.sig_hash(),
            B256::from_str(out["sigHash"].as_str().unwrap()).unwrap(),
            "{name}: sigHash"
        );
        assert_eq!(
            tx.mandatory_gas(),
            int(&out["mandatoryGas"]),
            "{name}: mandatoryGas"
        );
        assert_eq!(
            tx.max_gas(),
            int(&out["totalGasLimit"]),
            "{name}: totalGasLimit"
        );

        // One random case's max cost overflows U256, where `max_cost` saturates.
        if let Ok(max_cost) = U256::from_str(out["maxCost"].as_str().unwrap()) {
            assert_eq!(tx.max_cost(U256::ZERO), max_cost, "{name}: maxCost");
            let blob_base_fee = U256::from_str(out["blobBaseFee"].as_str().unwrap()).unwrap();
            let at_blob_fee =
                U256::from_str(out["maxCostAtBlobBaseFee"].as_str().unwrap()).unwrap();
            assert_eq!(
                tx.max_cost(blob_base_fee),
                at_blob_fee,
                "{name}: maxCostAtBlobBaseFee"
            );
        }
    }
    assert_eq!(checked, 12);
}

#[test]
fn signs_sig_hash() {
    let signer = PrivateKeySigner::random();
    let mut tx = TxFrame {
        chain_id: 8141,
        nonce_keys: vec![U256::from(7)],
        frames: vec![Frame {
            mode: FrameMode::Verify,
            data: Bytes::from_static(b"frame"),
            ..Frame::default()
        }],
        signatures: vec![FrameSignature::default()],
        ..TxFrame::default()
    };

    tx.sign_secp256k1(0, &signer).unwrap();
    let sig = &tx.signatures[0];
    assert_eq!(sig.scheme, SignatureScheme::Secp256k1);
    assert_eq!(sig.signer, signer.address());
    assert_eq!(sig.signature.len(), 65);
    assert!(sig.signature[0] <= 1);

    // Signing doesn't change the hash: empty-msg signatures are elided from it.
    let signature =
        Signature::from_raw(&[&sig.signature[1..], &[sig.signature[0] + 27]].concat()).unwrap();
    let recovered = signature
        .recover_address_from_prehash(&tx.sig_hash())
        .unwrap();
    assert_eq!(recovered, signer.address());
}

fn parse_tx(tx: &Value) -> Option<TxFrame> {
    Some(TxFrame {
        chain_id: int(&tx["chainId"]) as u64,
        nonce_keys: array(&tx["nonceKeys"]).map(u256).collect(),
        nonce_seq: int(&tx["nonceSeq"]) as u64,
        sender: address(&tx["sender"]),
        frames: array(&tx["frames"])
            .map(parse_frame)
            .collect::<Option<_>>()?,
        signatures: array(&tx["signatures"])
            .map(parse_signature)
            .collect::<Option<_>>()?,
        max_priority_fee_per_gas: u256(&tx["maxPriorityFee"]),
        max_fee_per_gas: u256(&tx["maxFee"]),
        max_fee_per_blob_gas: u256(&tx["maxBlobFee"]),
        blob_versioned_hashes: array(&tx["blobHashes"])
            .map(|h| B256::try_from(&bytes(h)[..]).ok())
            .collect::<Option<_>>()?,
    })
}

fn parse_frame(frame: &Value) -> Option<Frame> {
    let mode = match int(&frame["mode"]) {
        0 => FrameMode::Default,
        1 => FrameMode::Verify,
        2 => FrameMode::Sender,
        _ => return None,
    };
    let flags = int(&frame["flags"]);
    let approve_scope = match flags & 0b11 {
        0 => ApproveScope::None,
        1 => ApproveScope::Payment,
        2 => ApproveScope::Execution,
        _ => ApproveScope::ExecutionAndPayment,
    };
    if flags > 0b111 {
        return None;
    }

    Some(Frame {
        mode,
        approve_scope,
        atomic_batch: flags & 0b100 != 0,
        target: (!frame["target"].is_null()).then(|| address(&frame["target"])),
        gas_limit: int(&frame["gasLimit"]) as u64,
        state_gas_limit: int(&frame["stateLimit"]) as u64,
        value: u256(&frame["value"]),
        data: bytes(&frame["data"]),
    })
}

fn parse_signature(sig: &Value) -> Option<FrameSignature> {
    let scheme = match int(&sig["scheme"]) {
        0 => SignatureScheme::Arbitrary,
        1 => SignatureScheme::Secp256k1,
        2 => SignatureScheme::P256,
        _ => return None,
    };
    Some(FrameSignature {
        scheme,
        signer: address(&sig["signer"]),
        msg: bytes(&sig["msg"]),
        signature: bytes(&sig["signature"]),
    })
}

fn array(v: &Value) -> impl Iterator<Item = &Value> {
    v.as_array().unwrap().iter()
}

/// A decimal string, or a JSON number.
fn int(v: &Value) -> u128 {
    v.as_str()
        .map_or_else(|| u128::from(v.as_u64().unwrap()), |s| s.parse().unwrap())
}

fn u256(v: &Value) -> U256 {
    U256::from_str(v.as_str().unwrap()).unwrap()
}

/// A decimal-encoded address.
fn address(v: &Value) -> Address {
    Address::from_word(u256(v).into())
}

/// Hex, or `{prefix, pattern, length}`: the prefix, then the pattern repeated to `length` bytes.
fn bytes(v: &Value) -> Bytes {
    if let Some(s) = v.as_str() {
        return hex::decode(s).unwrap().into();
    }
    let prefix = v
        .get("prefix")
        .map(|p| hex::decode(p.as_str().unwrap()).unwrap())
        .unwrap_or_default();
    let pattern = hex::decode(v["pattern"].as_str().unwrap()).unwrap();
    let length = v["length"].as_u64().unwrap() as usize;
    (0..length)
        .map(|i| {
            if i < prefix.len() {
                prefix[i]
            } else {
                pattern[(i - prefix.len()) % pattern.len()]
            }
        })
        .collect()
}

/// A hegota devnet receipt for a `TintFrame` sponsored unshield, trimmed to two logs.
#[test]
fn decodes_receipt() {
    let receipt: FrameTxReceipt =
        serde_json::from_str(include_str!("fixtures/receipt.json")).unwrap();
    assert_eq!(receipt.status, TxStatus::Succeeded);
    let statuses: Vec<_> = receipt.frame_receipts.iter().map(|f| f.status).collect();
    assert_eq!(statuses, [FrameStatus::Succeeded, FrameStatus::Succeeded]);
    assert_eq!(receipt.payer, Some(receipt.logs[0].address()));
}

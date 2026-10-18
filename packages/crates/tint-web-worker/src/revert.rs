//! Decodes the revert data in RPC errors against the contracts' ABIs.

use alloy::{
    dyn_abi::{DynSolValue, ErrorExt},
    json_abi::JsonAbi,
    primitives::hex,
    rpc::json_rpc::ErrorPayload,
    sol_types::decode_revert_reason,
    transports::TransportError,
};

/// Forge artifacts whose custom errors reverts are decoded against. Tint's
/// includes the errors of the libraries it uses, such as `SafeERC20`'s, and
/// `MockToken`'s includes the standard ERC20 errors.
const ARTIFACTS: [&str; 2] = [
    include_str!("../../../contracts/out/Tint.sol/Tint.json"),
    include_str!("../../../contracts/out/MockToken.sol/MockToken.json"),
];

/// `err`, with any revert data it carries decoded.
pub fn describe(err: TransportError) -> anyhow::Error {
    let Some(data) = err.as_error_resp().and_then(ErrorPayload::as_revert_data) else {
        return err.into();
    };
    let reason = custom_error(&data)
        .or_else(|| decode_revert_reason(&data))
        .unwrap_or_else(|| format!("unknown error {}", hex::encode_prefixed(&data)));
    anyhow::anyhow!("execution reverted: {reason}")
}

/// Decodes `data` as a custom error from [`ARTIFACTS`], e.g.
/// `SafeERC20FailedOperation(token: 0x…)`.
fn custom_error(data: &[u8]) -> Option<String> {
    let selector = data.get(..4)?;
    let error = ARTIFACTS
        .iter()
        .filter_map(|artifact| abi(artifact))
        .flat_map(|abi| abi.errors.into_values().flatten())
        .find(|e| e.selector().as_slice() == selector)?;
    let decoded = error.decode_error(data).ok()?;

    let args: Vec<String> = error
        .inputs
        .iter()
        .zip(&decoded.body)
        .map(|(param, value)| format!("{}: {}", param.name, format_value(value)))
        .collect();
    Some(format!("{}({})", error.name, args.join(", ")))
}

fn abi(artifact: &str) -> Option<JsonAbi> {
    let artifact: serde_json::Value = serde_json::from_str(artifact).ok()?;
    serde_json::from_value(artifact.get("abi")?.clone()).ok()
}

fn format_value(value: &DynSolValue) -> String {
    match value {
        DynSolValue::Address(a) => a.to_string(),
        DynSolValue::Uint(u, _) => u.to_string(),
        DynSolValue::Int(i, _) => i.to_string(),
        DynSolValue::Bool(b) => b.to_string(),
        DynSolValue::FixedBytes(word, size) => hex::encode_prefixed(&word[..*size]),
        DynSolValue::Bytes(bytes) => hex::encode_prefixed(bytes),
        DynSolValue::String(s) => format!("{s:?}"),
        other => format!("{other:?}"),
    }
}

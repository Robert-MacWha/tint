//! Persists synced snapshots in the browser's Cache API, so reloading the
//! page doesn't refetch every event.

use alloy::primitives::{Address, keccak256};
use anyhow::Context;
use js_sys::Uint8Array;
use tint::syncer::Snapshot;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Cache, Response};

use crate::{artifacts::js_error, scope};

/// Bump the version whenever the stored encoding or key scheme changes, so
/// entries in an old format are never decoded.
const CACHE: &str = "tint-web-snapshots-v1";

/// The snapshot stored for `tint` on `rpc_url`, if any.
pub async fn load(rpc_url: &str, tint: Address) -> anyhow::Result<Option<Snapshot>> {
    let cached = JsFuture::from(cache().await?.match_with_str(&key(rpc_url, tint)))
        .await
        .map_err(|e| js_error(&e))?;
    if cached.is_undefined() {
        return Ok(None);
    }

    let response: Response = cached.dyn_into().map_err(|e| js_error(&e))?;
    let buffer = JsFuture::from(response.array_buffer().map_err(|e| js_error(&e))?)
        .await
        .map_err(|e| js_error(&e))?;
    let snapshot = postcard::from_bytes(&Uint8Array::new(&buffer).to_vec())
        .context("decoding stored snapshot")?;
    Ok(Some(snapshot))
}

/// Stores `snapshot` for its deployment on `rpc_url`, replacing any stored.
pub async fn save(rpc_url: &str, snapshot: &Snapshot) -> anyhow::Result<()> {
    let bytes = postcard::to_stdvec(snapshot).context("encoding snapshot")?;
    // Wasm memory is a SharedArrayBuffer with threads enabled, which Response
    // rejects, so copy the bytes out of it.
    let body = Uint8Array::new_from_slice(&bytes);
    let response = Response::new_with_opt_buffer_source(Some(&body)).map_err(|e| js_error(&e))?;
    JsFuture::from(
        cache()
            .await?
            .put_with_str(&key(rpc_url, snapshot.tint), &response),
    )
    .await
    .map_err(|e| js_error(&e))?;
    Ok(())
}

pub async fn delete(rpc_url: &str, tint: Address) -> anyhow::Result<()> {
    JsFuture::from(cache().await?.delete_with_str(&key(rpc_url, tint)))
        .await
        .map_err(|e| js_error(&e))?;
    Ok(())
}

async fn cache() -> anyhow::Result<Cache> {
    let caches = scope().caches().map_err(|e| js_error(&e))?;
    JsFuture::from(caches.open(CACHE))
        .await
        .map_err(|e| js_error(&e))?
        .dyn_into()
        .map_err(|e| js_error(&e))
}

/// The cache entry for `tint` on `rpc_url`. The Cache API keys entries by
/// URL, so this is a same-origin path the page never serves.
fn key(rpc_url: &str, tint: Address) -> String {
    format!("/snapshots/{}", keccak256(format!("{rpc_url} {tint}")))
}

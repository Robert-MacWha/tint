use anyhow::Context;
use ark_bn254::Bn254;
use ark_groth16::ProvingKey;
use ark_serialize::CanonicalDeserialize;
use js_sys::Uint8Array;
use tint_groth16::{
    artifacts::Artifacts,
    serde::{deserialize_matrices, deserialize_vk},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

use crate::{progress, scope};

/// Fetches and deserializes the join-split proving artifacts the page serves.
pub async fn fetch_artifacts(id: u64) -> anyhow::Result<Artifacts<Bn254>> {
    progress(id, "Downloading proving key");
    let pk = fetch_bytes("/artifacts/proving_key.bin").await?;
    let matrices = fetch_bytes("/artifacts/matrices.bin").await?;
    let vk = fetch_bytes("/artifacts/verifying_key.bin").await?;

    progress(id, "Loading proving key");
    // The page serves its own artifacts, so skip the curve checks, which
    // take ~30s single-threaded.
    let pk = ProvingKey::<Bn254>::deserialize_uncompressed_unchecked(pk.as_slice())
        .context("deserializing proving key")?;
    Ok(Artifacts {
        matrices: deserialize_matrices(&matrices).context("deserializing matrices")?,
        pk,
        vk: deserialize_vk(&vk).context("deserializing verifying key")?,
    })
}

async fn fetch_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let response: Response = JsFuture::from(scope().fetch_with_str(url))
        .await
        .map_err(|e| js_error(&e))?
        .dyn_into()
        .map_err(|e| js_error(&e))?;
    if !response.ok() {
        anyhow::bail!("fetching {url}: HTTP {}", response.status());
    }
    let buffer = JsFuture::from(response.array_buffer().map_err(|e| js_error(&e))?)
        .await
        .map_err(|e| js_error(&e))?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

fn js_error(value: &JsValue) -> anyhow::Error {
    anyhow::anyhow!("{value:?}")
}

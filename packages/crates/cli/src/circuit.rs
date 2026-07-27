use std::io::Read;

use ark_bn254::Bn254;
use ark_groth16::{ProvingKey, VerifyingKey};
use ark_serialize::CanonicalDeserialize;

const PROVING_KEY: &[u8] = include_bytes!("../../tint/artifacts/proving_key.bin.br");
const VERIFYING_KEY: &[u8] = include_bytes!("../../tint/artifacts/verifying_key.bin.br");

const BUFFER_SIZE: usize = 4096;

/// Loads the embedded Groth16 proving/verifying keys for the `JoinSplit` circuit.
pub fn load_keys() -> anyhow::Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>)> {
    let pk = ProvingKey::<Bn254>::deserialize_uncompressed_unchecked(&decompress(PROVING_KEY)?[..])?;
    let vk =
        VerifyingKey::<Bn254>::deserialize_uncompressed_unchecked(&decompress(VERIFYING_KEY)?[..])?;

    Ok((pk, vk))
}

fn decompress(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    brotli::Decompressor::new(data, BUFFER_SIZE).read_to_end(&mut out)?;
    Ok(out)
}

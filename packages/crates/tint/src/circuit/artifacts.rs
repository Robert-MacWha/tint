//! Compression + (de)serialization helpers for circuit artifacts.

use ark_bn254::Bn254;
use ark_groth16::{ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

use crate::circuit::matrices::Matrices;

// const BUFFER_SIZE: usize = 4096;
// const QUALITY: u32 = 11;
// const LGWIN: u32 = 22;

#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("serialization error: {0}")]
    Serialization(#[from] ark_serialize::SerializationError),
    #[error("postcard error: {0}")]
    Postcard(#[from] postcard::Error),
    #[error("compression error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn serialize_proving_key(pk: &ProvingKey<Bn254>) -> Result<Vec<u8>, ArtifactError> {
    serialize_canonical(pk)
}

pub fn deserialize_proving_key(bytes: &[u8]) -> Result<ProvingKey<Bn254>, ArtifactError> {
    deserialize_canonical(bytes)
}

pub fn serialize_verifying_key(vk: &VerifyingKey<Bn254>) -> Result<Vec<u8>, ArtifactError> {
    serialize_canonical(vk)
}

pub fn deserialize_verifying_key(bytes: &[u8]) -> Result<VerifyingKey<Bn254>, ArtifactError> {
    deserialize_canonical(bytes)
}

pub fn serialize_matrices(matrices: &Matrices) -> Result<Vec<u8>, ArtifactError> {
    serialize_postcard(matrices)
}

pub fn deserialize_matrices(bytes: &[u8]) -> Result<Matrices, ArtifactError> {
    deserialize_postcard(bytes)
}

/// Serializes `value` with `ark-serialize` and brotli-compresses the result.
fn serialize_canonical<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>, ArtifactError> {
    let mut bytes = Vec::new();
    value.serialize_uncompressed(&mut bytes)?;
    Ok(bytes)
}

/// Brotli-decompresses `bytes` and deserializes with `ark-serialize`.
fn deserialize_canonical<T: CanonicalDeserialize>(bytes: &[u8]) -> Result<T, ArtifactError> {
    Ok(T::deserialize_uncompressed(&bytes[..])?)
}

/// Serializes `value` with `postcard` and brotli-compresses the result.
fn serialize_postcard<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, ArtifactError> {
    let bytes = postcard::to_stdvec(value)?;
    Ok(bytes)
}

/// Brotli-decompresses `bytes` and deserializes with `postcard`.
fn deserialize_postcard<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, ArtifactError> {
    Ok(postcard::from_bytes(&bytes)?)
}

// fn compress(data: &[u8]) -> Result<Vec<u8>, ArtifactError> {
//     let mut compressed = Vec::new();
//     CompressorWriter::new(&mut compressed, BUFFER_SIZE, QUALITY, LGWIN).write_all(data)?;
//     Ok(compressed)
// }

// fn decompress(data: &[u8]) -> Result<Vec<u8>, ArtifactError> {
//     let mut decompressed = Vec::new();
//     Decompressor::new(data, BUFFER_SIZE).read_to_end(&mut decompressed)?;
//     Ok(decompressed)
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_round_trip() {
        let value = ark_bn254::Fr::from(1234567u64);
        let bytes = serialize_canonical(&value).unwrap();
        let decoded: ark_bn254::Fr = deserialize_canonical(&bytes).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn postcard_round_trip() {
        let value: Vec<u32> = vec![1, 2, 3, 4, 5];
        let bytes = serialize_postcard(&value).unwrap();
        let decoded: Vec<u32> = deserialize_postcard(&bytes).unwrap();
        assert_eq!(value, decoded);
    }
}

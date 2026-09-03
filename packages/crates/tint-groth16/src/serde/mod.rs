use ark_bn254::Bn254;
use ark_ff::PrimeField;
use ark_groth16::{ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

use crate::matrices::Matrices;

pub mod field;

#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    #[error("serialization error: {0}")]
    Serialization(#[from] ark_serialize::SerializationError),
    #[error("postcard error: {0}")]
    Postcard(#[from] postcard::Error),
    #[error("compression error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn serialize_pk(pk: &ProvingKey<Bn254>) -> Result<Vec<u8>, SerializationError> {
    serialize_canonical(pk)
}

pub fn deserialize_pk(bytes: &[u8]) -> Result<ProvingKey<Bn254>, SerializationError> {
    deserialize_canonical(bytes)
}

pub fn serialize_vk(vk: &VerifyingKey<Bn254>) -> Result<Vec<u8>, SerializationError> {
    serialize_canonical(vk)
}

pub fn deserialize_vk(bytes: &[u8]) -> Result<VerifyingKey<Bn254>, SerializationError> {
    deserialize_canonical(bytes)
}

pub fn serialize_matrices<F: PrimeField>(
    matrices: &Matrices<F>,
) -> Result<Vec<u8>, SerializationError> {
    serialize_postcard(matrices)
}

pub fn deserialize_matrices<F: PrimeField>(
    bytes: &[u8],
) -> Result<Matrices<F>, SerializationError> {
    deserialize_postcard(bytes)
}

/// Serializes `value` with `ark-serialize` and brotli-compresses the result.
fn serialize_canonical<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>, SerializationError> {
    let mut bytes = Vec::new();
    value.serialize_uncompressed(&mut bytes)?;
    Ok(bytes)
}

/// Brotli-decompresses `bytes` and deserializes with `ark-serialize`.
fn deserialize_canonical<T: CanonicalDeserialize>(bytes: &[u8]) -> Result<T, SerializationError> {
    Ok(T::deserialize_uncompressed(bytes)?)
}

/// Serializes `value` with `postcard` and brotli-compresses the result.
fn serialize_postcard<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, SerializationError> {
    let bytes = postcard::to_stdvec(value)?;
    Ok(bytes)
}

/// Brotli-decompresses `bytes` and deserializes with `postcard`.
fn deserialize_postcard<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
) -> Result<T, SerializationError> {
    Ok(postcard::from_bytes(bytes)?)
}

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

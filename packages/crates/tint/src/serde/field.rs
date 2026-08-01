use ark_ff::{BigInteger, PrimeField};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

pub struct FieldAsBytes;

impl<F: PrimeField> SerializeAs<F> for FieldAsBytes {
    fn serialize_as<S>(source: &F, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = source.into_bigint().to_bytes_be();
        bytes.serialize(serializer)
    }
}

impl<'de, F: PrimeField> DeserializeAs<'de, F> for FieldAsBytes {
    fn deserialize_as<D>(deserializer: D) -> Result<F, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Ok(F::from_be_bytes_mod_order(&bytes))
    }
}

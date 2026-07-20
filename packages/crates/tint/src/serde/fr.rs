use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

pub struct FrAsBytes;

impl SerializeAs<Fr> for FrAsBytes {
    fn serialize_as<S: Serializer>(value: &Fr, serializer: S) -> Result<S::Ok, S::Error> {
        let bytes = value.into_bigint().to_bytes_be();
        bytes.serialize(serializer)
    }
}

impl<'de> DeserializeAs<'de, Fr> for FrAsBytes {
    fn deserialize_as<D: Deserializer<'de>>(deserializer: D) -> Result<Fr, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Ok(Fr::from_be_bytes_mod_order(&bytes))
    }
}

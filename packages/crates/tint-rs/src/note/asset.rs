use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub Address);

impl From<Address> for AssetId {
    fn from(address: Address) -> Self {
        AssetId(address)
    }
}

impl From<AssetId> for Address {
    fn from(asset: AssetId) -> Self {
        asset.0
    }
}

impl From<AssetId> for Fr {
    fn from(asset: AssetId) -> Self {
        let buf = asset.0.into_word().0;
        Fr::from_be_bytes_mod_order(&buf)
    }
}

impl From<Fr> for AssetId {
    fn from(fr: Fr) -> Self {
        let bytes = fr.into_bigint().to_bytes_be();
        let mut buf = [0u8; 32];
        buf[32 - bytes.len()..].copy_from_slice(&bytes);
        AssetId(Address::from_word(buf.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_id_to_fr() {
        let address = Address::from_word([1u8; 32].into());
        let asset_id = AssetId(address);
        let fr: Fr = asset_id.into();
        let asset_id_back: AssetId = fr.into();
        assert_eq!(asset_id, asset_id_back);
    }
}

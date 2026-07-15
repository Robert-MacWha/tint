use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_ff::PrimeField;
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
        let mut bytes = asset.0.into_array();
        bytes.reverse();
        Fr::from_be_bytes_mod_order(&bytes)
    }
}

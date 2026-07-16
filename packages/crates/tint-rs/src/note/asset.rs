use alloy_primitives::Address;
use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

use crate::indexer::{address_to_fr, fr_to_address};

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
        address_to_fr(asset.0)
    }
}

impl From<Fr> for AssetId {
    fn from(fr: Fr) -> Self {
        AssetId(fr_to_address(fr))
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

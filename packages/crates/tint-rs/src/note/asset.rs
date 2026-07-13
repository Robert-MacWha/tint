use alloy_primitives::Address;
use ark_bn254::Fr;
use ark_ff::PrimeField;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
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

/// Matches `ProofLib.assetToFr` in `Tint.sol`: the raw 20 address bytes,
/// byte-reversed and read as a big-endian integer (Solidity byte-reverses
/// the address in a loop, then uses it as-is; this mirrors that literally
/// instead of relying on the mathematically-equivalent little-endian
/// interpretation of the unreversed bytes), reduced mod the field order.
impl From<AssetId> for Fr {
    fn from(asset: AssetId) -> Self {
        let mut bytes = asset.0.into_array();
        bytes.reverse();
        Fr::from_be_bytes_mod_order(&bytes)
    }
}

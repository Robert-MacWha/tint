use alloy_primitives::{Address, keccak256};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AssetId(pub Address);

impl AssetId {
    pub fn new(address: Address) -> Self {
        AssetId(address)
    }

    pub fn hash(&self) -> u128 {
        let bytes = self.0.as_slice();
        let hash = keccak256(bytes);
        u128::from_le_bytes(hash[..16].try_into().unwrap())
    }

    pub fn to_fr(&self) -> ark_bn254::Fr {
        let hash = self.hash();
        ark_bn254::Fr::from(hash)
    }
}

impl Default for AssetId {
    fn default() -> Self {
        AssetId(Address::default())
    }
}

impl From<Address> for AssetId {
    fn from(address: Address) -> Self {
        AssetId::new(address)
    }
}

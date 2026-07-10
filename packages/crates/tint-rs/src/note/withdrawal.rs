use alloy_primitives::Address;
use ark_bn254::Fr;

use crate::note::asset::AssetId;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Withdrawal {
    pub asset: AssetId,
    pub amount: u128,
    pub to: Address,
}

impl Withdrawal {
    pub fn new(asset: AssetId, amount: u128, to: Address) -> Self {
        Withdrawal { asset, amount, to }
    }

    pub fn asset_fr(&self) -> Fr {
        self.asset.to_fr()
    }

    pub fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }
}

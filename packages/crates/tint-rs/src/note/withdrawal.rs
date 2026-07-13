use ark_bn254::Fr;

use crate::note::asset::AssetId;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Withdrawal {
    pub asset: AssetId,
    pub amount: u128,
}

impl Withdrawal {
    pub fn new(asset: AssetId, amount: u128) -> Self {
        Withdrawal { asset, amount }
    }

    pub fn asset_fr(&self) -> Fr {
        Fr::from(self.asset)
    }

    pub fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }
}

use alloy_primitives::Address;
use ark_bn254::Fr;

use crate::{circuit::poseidon2::poseidon2_compress, fr::address_to_fr, note::asset::AssetId};

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Withdrawal {
    pub asset: AssetId,
    pub amount: u128,
    pub recipient: Address,
}

impl Withdrawal {
    #[must_use]
    pub fn new(asset: AssetId, amount: u128, recipient: Address) -> Self {
        Withdrawal {
            asset,
            amount,
            recipient,
        }
    }

    #[must_use]
    pub fn asset_fr(&self) -> Fr {
        Fr::from(self.asset)
    }

    #[must_use]
    pub fn amount_fr(&self) -> Fr {
        Fr::from(self.amount)
    }

    #[must_use]
    pub fn recipient_fr(&self) -> Fr {
        address_to_fr(self.recipient)
    }

    #[must_use]
    pub fn hash(&self) -> Fr {
        poseidon2_compress(&[self.asset_fr(), self.amount_fr(), self.recipient_fr()])
    }
}

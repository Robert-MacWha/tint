use alloy::primitives::Address;

pub struct Withdraw {
    pub asset: Address,
    pub amount: u128,
    pub to: Address,
}

impl Withdraw {
    pub fn new(asset: Address, amount: u128, to: Address) -> Self {
        Withdraw { asset, amount, to }
    }
}

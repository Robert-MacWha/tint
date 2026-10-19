//! The signer's public balances, and a faucet for them on Tenderly virtual
//! testnets.

use alloy::{
    primitives::{Address, U256},
    providers::{DynProvider, Provider},
    sol,
};
use tint_web_protocol::{PublicBalance, PublicState};

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address owner) external view returns (uint256);
    }
}

/// `owner`'s native and `tokens` balances.
pub async fn state(
    provider: &DynProvider,
    owner: Address,
    tokens: &[Address],
) -> anyhow::Result<PublicState> {
    let eth = provider.get_balance(owner).await?;
    let mut balances = Vec::with_capacity(tokens.len());
    for &token in tokens {
        let amount = IERC20::new(token, provider).balanceOf(owner).call().await?;
        balances.push(PublicBalance {
            token,
            amount: amount.saturating_to(),
        });
    }

    Ok(PublicState {
        address: owner,
        eth: eth.saturating_to(),
        balances,
    })
}

/// Sets `owner`'s native and token balances with Tenderly's cheatcodes.
pub async fn fund(
    provider: &DynProvider,
    owner: Address,
    eth: u128,
    tokens: &[PublicBalance],
) -> anyhow::Result<()> {
    let _: serde_json::Value = provider
        .raw_request("tenderly_setBalance".into(), (owner, U256::from(eth)))
        .await?;
    for balance in tokens {
        let _: serde_json::Value = provider
            .raw_request(
                "tenderly_setErc20Balance".into(),
                (balance.token, owner, U256::from(balance.amount)),
            )
            .await?;
    }
    Ok(())
}

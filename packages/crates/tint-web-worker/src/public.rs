//! The signer's public balances, and a faucet for them on Tenderly virtual
//! testnets.

use alloy::{
    primitives::{Address, U256},
    providers::{DynProvider, Provider},
    sol,
};
use futures::future::{try_join, try_join_all};
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
    let balances = try_join_all(tokens.iter().map(|&token| async move {
        let amount = IERC20::new(token, provider).balanceOf(owner).call().await?;
        anyhow::Ok(PublicBalance {
            token,
            amount: amount.saturating_to(),
        })
    }));
    let (eth, balances) = try_join(
        async { anyhow::Ok(provider.get_balance(owner).await?) },
        balances,
    )
    .await?;

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
    let set_eth = async {
        let _: serde_json::Value = provider
            .raw_request("tenderly_setBalance".into(), (owner, U256::from(eth)))
            .await?;
        anyhow::Ok(())
    };
    let set_tokens = try_join_all(tokens.iter().map(|balance| async move {
        let _: serde_json::Value = provider
            .raw_request(
                "tenderly_setErc20Balance".into(),
                (balance.token, owner, U256::from(balance.amount)),
            )
            .await?;
        anyhow::Ok(())
    }));
    try_join(set_eth, set_tokens).await?;
    Ok(())
}

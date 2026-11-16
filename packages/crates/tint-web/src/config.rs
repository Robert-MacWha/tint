//! The deployment tint-web runs against, and the tokens it offers.

use alloy_primitives::{Address, address};
use serde::Deserialize;
use tint_web_protocol::{PublicBalance, Sponsor};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

/// Where the page serves its [`Deployment`]: written by `just web` in
/// development, and by the NixOS module in production.
const DEPLOYMENT_URL: &str = "/config.json";

/// The chain, contracts, and services tint-web uses, loaded at runtime so one
/// build serves any deployment.
#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub rpc_url: String,
    /// The explorer transactions link to, as `{url}/tx/{hash}`.
    pub explorer_url: Option<String>,
    pub tint: Address,
    pub paymaster: Address,
    pub bundler_url: String,
}

/// A token the UI offers, and how much of it the faucet gives.
pub struct Token {
    pub symbol: &'static str,
    pub address: Address,
    pub faucet: u128,
}

pub const TOKENS: [Token; 2] = [
    Token {
        symbol: "WETH",
        address: address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        faucet: 250_000_000_000_000_000,
    },
    Token {
        symbol: "USDC",
        address: address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        faucet: 1_000_000_000,
    },
];

/// The native balance, in wei, the faucet gives.
pub const FAUCET_ETH: u128 = 250_000_000_000_000_000;

impl Deployment {
    pub async fn fetch() -> Result<Self, String> {
        let window = web_sys::window().ok_or("no window")?;
        let response: Response = JsFuture::from(window.fetch_with_str(DEPLOYMENT_URL))
            .await
            .map_err(|e| js_error(&e))?
            .dyn_into()
            .map_err(|e| js_error(&e))?;
        if !response.ok() {
            return Err(format!(
                "fetching {DEPLOYMENT_URL}: HTTP {}",
                response.status()
            ));
        }
        let text = JsFuture::from(response.text().map_err(|e| js_error(&e))?)
            .await
            .map_err(|e| js_error(&e))?
            .as_string()
            .ok_or("deployment isn't text")?;
        serde_json::from_str(&text).map_err(|e| format!("invalid {DEPLOYMENT_URL}: {e}"))
    }

    pub fn sponsor(&self) -> Sponsor {
        Sponsor {
            paymaster: self.paymaster,
            bundler_url: self.bundler_url.clone(),
        }
    }
}

/// The balances the faucet sets each token to.
pub fn faucet_tokens() -> Vec<PublicBalance> {
    TOKENS
        .iter()
        .map(|t| PublicBalance {
            token: t.address,
            amount: t.faucet,
        })
        .collect()
}

/// `token`'s symbol, if it's one of [`TOKENS`].
pub fn symbol(token: Address) -> Option<&'static str> {
    TOKENS.iter().find(|t| t.address == token).map(|t| t.symbol)
}

fn js_error(value: &JsValue) -> String {
    format!("{value:?}")
}

//! The deployment tint-web is built for.
//!
//! URLs and addresses come from the environment at build time (`just web`
//! loads them from `just env`), so the RPC URL stays out of the repository.

use alloy_primitives::{Address, address};
use tint_web_protocol::{PublicBalance, Sponsor};

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

pub fn rpc_url() -> Result<&'static str, String> {
    built_with("RPC_URL", option_env!("RPC_URL"))
}

/// The explorer transactions link to, as `{url}/tx/{hash}`, if built with one.
pub fn explorer_url() -> Option<&'static str> {
    option_env!("EXPLORER_URL")
}

pub fn tint() -> Result<Address, String> {
    parse_address("TINT_ADDRESS", option_env!("TINT_ADDRESS"))
}

pub fn sponsor() -> Result<Sponsor, String> {
    Ok(Sponsor {
        paymaster: parse_address("PAYMASTER_ADDRESS", option_env!("PAYMASTER_ADDRESS"))?,
        bundler_url: built_with("BUNDLER_URL", option_env!("BUNDLER_URL"))?.to_string(),
    })
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

fn parse_address(name: &str, value: Option<&'static str>) -> Result<Address, String> {
    built_with(name, value)?
        .parse()
        .map_err(|_| format!("{name} isn't an address"))
}

fn built_with(name: &str, value: Option<&'static str>) -> Result<&'static str, String> {
    value.ok_or_else(|| format!("tint-web was built without {name}; build it with `just web`"))
}

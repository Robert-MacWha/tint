//! Settings, persisted in the browser's `localStorage`.

use alloy_primitives::{Address, B256};
use alloy_signer_local::PrivateKeySigner;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tint_web_protocol::{Connection, Sponsor};
use web_sys::Storage;

pub const SETTINGS_KEY: &str = "tint-web/settings";

/// The chain, signer, known tokens, paymaster, and explorer, as typed by the user.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub rpc_url: String,
    pub tint: String,
    pub private_key: String,
    /// Known tokens, one `SYMBOL 0xaddress` per line.
    pub tokens: String,
    pub paymaster: String,
    pub bundler_url: String,
    /// The block explorer transactions link to, as `{explorer_url}/tx/{hash}`.
    pub explorer_url: String,
}

/// A known token, as listed in [`Settings::tokens`].
#[derive(Clone, PartialEq)]
pub struct Token {
    pub symbol: String,
    pub address: Address,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8545".to_string(),
            tint: String::new(),
            private_key: String::new(),
            tokens: String::new(),
            paymaster: String::new(),
            bundler_url: "http://127.0.0.1:4337".to_string(),
            explorer_url:
                "https://dashboard.tenderly.co/explorer/vnet/c864f850-a1ce-475a-b196-2b1b14573e7a"
                    .to_string(),
        }
    }
}

impl Settings {
    /// The signer's address, if the private key is valid.
    pub fn signer(&self) -> Option<Address> {
        let key: B256 = self.private_key.trim().parse().ok()?;
        Some(PrivateKeySigner::from_bytes(&key).ok()?.address())
    }

    pub fn connection(&self) -> Result<Connection, String> {
        Ok(Connection {
            rpc_url: self.rpc_url.trim().to_string(),
            tint: self
                .tint
                .trim()
                .parse()
                .map_err(|_| "invalid Tint address")?,
            private_key: self
                .private_key
                .trim()
                .parse()
                .map_err(|_| "invalid private key")?,
        })
    }

    pub fn sponsor(&self) -> Result<Sponsor, String> {
        Ok(Sponsor {
            paymaster: self
                .paymaster
                .trim()
                .parse()
                .map_err(|_| "invalid paymaster address")?,
            bundler_url: self.bundler_url.trim().to_string(),
        })
    }

    /// The parseable lines of [`Self::tokens`].
    pub fn tokens(&self) -> Vec<Token> {
        self.tokens
            .lines()
            .filter_map(|line| {
                let (symbol, address) = line.trim().split_once(char::is_whitespace)?;
                Some(Token {
                    symbol: symbol.to_string(),
                    address: address.trim().parse().ok()?,
                })
            })
            .collect()
    }

    /// `token`'s symbol, if it's a known token.
    pub fn symbol(&self, token: Address) -> Option<String> {
        self.tokens()
            .into_iter()
            .find(|t| t.address == token)
            .map(|t| t.symbol)
    }
}

/// Loads the value stored under `key`, or its default if there is none.
pub fn load<T: DeserializeOwned + Default>(key: &str) -> T {
    storage()
        .and_then(|s| s.get_item(key).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save<T: Serialize>(key: &str, value: &T) {
    let (Some(storage), Ok(json)) = (storage(), serde_json::to_string(value)) else {
        return;
    };
    let _ = storage.set_item(key, &json);
}

fn storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok()?
}

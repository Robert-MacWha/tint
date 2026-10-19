//! Settings, persisted in the browser's `localStorage`.

use alloy_primitives::{Address, B256};
use alloy_signer_local::PrivateKeySigner;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tint_web_protocol::Connection;
use web_sys::Storage;

use crate::config;

pub const SETTINGS_KEY: &str = "tint-web/settings";

/// The signer's private key, as typed by the user.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub private_key: String,
}

impl Settings {
    /// The signer's address, if the private key is valid.
    pub fn signer(&self) -> Option<Address> {
        let key: B256 = self.private_key.trim().parse().ok()?;
        Some(PrivateKeySigner::from_bytes(&key).ok()?.address())
    }

    pub fn connection(&self) -> Result<Connection, String> {
        Ok(Connection {
            rpc_url: config::rpc_url()?.to_string(),
            tint: config::tint()?,
            private_key: self
                .private_key
                .trim()
                .parse()
                .map_err(|_| "invalid private key")?,
        })
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

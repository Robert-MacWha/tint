use alloy_primitives::hex;
use alloy_signer_local::PrivateKeySigner;
use dioxus::prelude::*;

use crate::storage::Settings;

/// The private key every request signs with.
#[component]
pub fn SettingsPanel(settings: Signal<Settings>) -> Element {
    rsx! {
        label { "Private key" }
        input {
            value: "{settings.read().private_key}",
            placeholder: "0x…",
            oninput: move |e| settings.write().private_key = e.value(),
        }
        p {
            button {
                onclick: move |_| {
                    let key = PrivateKeySigner::random().to_bytes();
                    settings.write().private_key = hex::encode_prefixed(key);
                },
                "New key"
            }
            " Testnet only. Never paste a real key."
        }
        if !settings.read().private_key.is_empty() {
            div { class: "mono",
                match settings.read().signer() {
                    Some(address) => rsx! { "Address: {address}" },
                    None => rsx! { "Invalid private key" },
                }
            }
        }
    }
}

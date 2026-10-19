use dioxus::prelude::*;

use crate::storage::Settings;

/// The RPC, deployment, signer, and known tokens every request uses, and
/// the paymaster sponsored requests use.
#[component]
pub fn SettingsPanel(settings: Signal<Settings>) -> Element {
    rsx! {
        h3 { "Settings" }
        label { "RPC URL" }
        input {
            value: "{settings.read().rpc_url}",
            oninput: move |e| settings.write().rpc_url = e.value(),
        }
        label { "Tint address" }
        input {
            value: "{settings.read().tint}",
            placeholder: "0x…",
            oninput: move |e| settings.write().tint = e.value(),
        }
        label { "Private key" }
        input {
            value: "{settings.read().private_key}",
            placeholder: "0x…",
            oninput: move |e| settings.write().private_key = e.value(),
        }
        if !settings.read().private_key.is_empty() {
            div { class: "mono",
                match settings.read().signer() {
                    Some(address) => rsx! { "Address: {address}" },
                    None => rsx! { "Invalid private key" },
                }
            }
        }
        label { "Tokens (one \"SYMBOL 0xaddress\" per line)" }
        textarea {
            rows: 4,
            value: "{settings.read().tokens}",
            oninput: move |e| settings.write().tokens = e.value(),
        }
        label { "Paymaster address (for sponsored operations)" }
        input {
            value: "{settings.read().paymaster}",
            placeholder: "0x…",
            oninput: move |e| settings.write().paymaster = e.value(),
        }
        label { "Bundler URL (for sponsored operations)" }
        input {
            value: "{settings.read().bundler_url}",
            oninput: move |e| settings.write().bundler_url = e.value(),
        }
        label { "Explorer URL (transactions link to URL/tx/HASH)" }
        input {
            value: "{settings.read().explorer_url}",
            oninput: move |e| settings.write().explorer_url = e.value(),
        }
    }
}

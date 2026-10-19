use dioxus::prelude::*;
use tint_web_protocol::{PublicState, RequestKind};

use crate::{ErrorMessage, account::TokenSymbol, config, storage::Settings, worker::WorkerClient};

/// The signer's public balances, with a faucet to top them up.
#[component]
pub fn PublicView() -> Element {
    let client = use_context::<WorkerClient>();
    let settings = use_context::<Signal<Settings>>();
    let ErrorMessage(error) = use_context();
    let state = use_signal(|| None::<PublicState>);
    let busy = use_signal(|| false);

    let run = use_callback(move |kind: Result<RequestKind, String>| {
        spawn(run_request(client.clone(), kind, state, busy, error));
    });
    let refresh = move || {
        run.call(
            settings
                .read()
                .connection()
                .map(|conn| RequestKind::Public {
                    conn,
                    tokens: config::TOKENS.iter().map(|t| t.address).collect(),
                }),
        );
    };
    use_hook(refresh);
    let faucet = move |_| {
        run.call(
            settings
                .read()
                .connection()
                .map(|conn| RequestKind::Faucet {
                    conn,
                    eth: config::FAUCET_ETH,
                    tokens: config::faucet_tokens(),
                }),
        );
    };

    let disabled = busy();
    rsx! {
        h2 { "Public" }
        p {
            button { disabled, onclick: move |_| refresh(), "Refresh" }
            " "
            button { disabled, onclick: faucet, "Faucet" }
            " Sets the balances below with Tenderly's cheatcodes."
        }
        if let Some(state) = state() {
            p { class: "mono", "Address: {state.address}" }
            table {
                thead {
                    tr {
                        th { "token" }
                        th { "amount (base units)" }
                    }
                }
                tbody {
                    tr {
                        td { "ETH" }
                        td { "{state.eth}" }
                    }
                    for balance in state.balances {
                        tr {
                            td {
                                TokenSymbol { token: balance.token }
                            }
                            td { "{balance.amount}" }
                        }
                    }
                }
            }
        }
    }
}

async fn run_request(
    client: WorkerClient,
    kind: Result<RequestKind, String>,
    mut state: Signal<Option<PublicState>>,
    mut busy: Signal<bool>,
    mut error: Signal<Option<String>>,
) {
    let kind = match kind {
        Ok(kind) => kind,
        Err(e) => {
            error.set(Some(e));
            return;
        }
    };

    busy.set(true);
    let result = client.public(kind).await;
    busy.set(false);
    match result {
        Ok(new_state) => {
            state.set(Some(new_state));
            error.set(None);
        }
        Err(e) => error.set(Some(e)),
    }
}

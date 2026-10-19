//! A browser UI for Tint. All tint work runs in a web worker (`tint-web-worker`),
//! which this talks to through [`WorkerClient`].

mod account;
mod accounts;
mod actions;
mod config;
mod public;
mod settings;
mod storage;
mod worker;

use dioxus::prelude::*;
use tint_web_protocol::AccountInfo;

use crate::{
    account::AccountView,
    accounts::{Accounts, Selected},
    config::Deployment,
    public::PublicView,
    settings::SettingsPanel,
    storage::{SETTINGS_KEY, Settings},
    worker::{LogEntry, WorkerClient},
};

/// The worker's latest progress update for an in-flight request.
#[derive(Clone, Copy)]
struct Status(Signal<Option<String>>);

/// The latest failed request's error.
#[derive(Clone, Copy)]
struct ErrorMessage(Signal<Option<String>>);

/// Every request, progress update, and outcome, oldest first.
#[derive(Clone, Copy)]
struct Log(Signal<Vec<LogEntry>>);

/// Bumped to recover the account list again, after a request that may have
/// changed which accounts are used.
#[derive(Clone, Copy)]
struct ReloadAccounts(Signal<u64>);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let status = use_context_provider(|| Status(Signal::new(None)));
    let log = use_context_provider(|| Log(Signal::new(Vec::new())));
    let client = use_hook(|| WorkerClient::spawn(status.0, log.0));
    let deployment = use_resource(Deployment::fetch);

    let client = match client {
        Ok(client) => client,
        Err(e) => {
            return rsx! {
                p { class: "error", "Failed to start the tint worker: {e}" }
            };
        }
    };
    match deployment() {
        None => rsx! {
            p { "Loading the deployment…" }
        },
        Some(Err(e)) => rsx! {
            p { class: "error", "Failed to load the deployment: {e}" }
        },
        Some(Ok(deployment)) => rsx! {
            Wallet { client, deployment }
        },
    }
}

#[component]
fn Wallet(client: WorkerClient, deployment: Deployment) -> Element {
    use_context_provider(|| client.clone());
    let deployment = use_context_provider(|| Signal::new(deployment));
    let Status(status) = use_context();
    let ErrorMessage(mut error) = use_context_provider(|| ErrorMessage(Signal::new(None)));
    let ReloadAccounts(reload) = use_context_provider(|| ReloadAccounts(Signal::new(0)));
    let settings = use_context_provider(|| Signal::new(storage::load::<Settings>(SETTINGS_KEY)));
    let mut accounts = use_signal(Vec::<AccountInfo>::new);
    // New users start here, to fund their key.
    let selected = use_signal(|| Selected::Public);
    // How many accounts to show, beyond those recovered as used.
    let min = use_signal(|| 1);
    let conn = use_memo(move || settings.read().connection(&deployment.read()).ok());

    use_effect(move || storage::save(SETTINGS_KEY, &*settings.read()));
    // Recovers the accounts whenever the connection changes.
    use_effect(move || {
        let (conn, min, _) = (conn(), min(), reload());
        let Some(conn) = conn else {
            accounts.set(Vec::new());
            return;
        };
        let client = client.clone();
        spawn(async move {
            match client.accounts(conn, min).await {
                Ok(recovered) => accounts.set(recovered),
                Err(e) => error.set(Some(e)),
            }
        });
    });

    let account = accounts
        .read()
        .iter()
        .find(|a| selected() == Selected::Account(a.index))
        .cloned();
    let signer = settings.read().signer();

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div { class: "container",
            aside { class: "sidebar",
                h1 { "Tint" }
                SettingsPanel { settings }
                Accounts { accounts, selected, min }
                WorkerLog {}
            }
            main { class: "content",
                p {
                    b { "Worker: " }
                    match status() {
                        Some(status) => rsx! { "{status}…" },
                        None => rsx! { "idle" },
                    }
                }
                if let Some(error) = error() {
                    p { class: "error", "Error: {error}" }
                }
                match (selected(), account, signer) {
                    // Keyed by address, so a new private key remounts it.
                    (Selected::Public, _, Some(signer)) => rsx! {
                        PublicView { key: "{signer}" }
                    },
                    (Selected::Account(_), Some(account), _) => rsx! {
                        AccountView { key: "{account.receiver}", account, accounts }
                    },
                    _ => rsx! {
                        p { "Select an account." }
                    },
                }
            }
        }
    }
}

#[component]
fn WorkerLog() -> Element {
    let Log(log) = use_context();
    rsx! {
        h3 { "Worker log" }
        div { class: "log",
            table {
                thead {
                    tr {
                        th { "time" }
                        th { "id" }
                        th { "request" }
                        th { "message" }
                    }
                }
                tbody {
                    for entry in log.read().iter().rev() {
                        tr {
                            td { "{entry.time}" }
                            td { "{entry.id}" }
                            td { "{entry.request}" }
                            td { "{entry.message}" }
                        }
                    }
                }
            }
        }
    }
}

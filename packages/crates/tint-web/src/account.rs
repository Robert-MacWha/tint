use alloy_primitives::Address;
use dioxus::prelude::*;
use tint_web_protocol::{
    AccountInfo, AccountState, Balance, GlobalEntry, HistoryEntry, HistoryKind, NoteInfo,
    RequestKind,
};

use crate::{
    ErrorMessage, ReloadAccounts,
    actions::{Action, DepositForm, TransferForm, WithdrawForm},
    storage::Settings,
    worker::WorkerClient,
};

/// An account's diagnostics, actions, balances, notes, and history.
#[component]
pub fn AccountView(account: AccountInfo, accounts: Signal<Vec<AccountInfo>>) -> Element {
    let client = use_context::<WorkerClient>();
    let settings = use_context::<Signal<Settings>>();
    let ErrorMessage(error) = use_context();
    let ReloadAccounts(reload) = use_context();
    let state = use_signal(|| None::<AccountState>);
    let busy = use_signal(|| false);
    let mut action = use_signal(|| None::<Action>);
    let index = account.index;

    let run = use_callback(move |kind: Result<RequestKind, String>| {
        spawn(run_request(
            client.clone(),
            kind,
            state,
            busy,
            error,
            reload,
        ));
    });
    let sync = move || {
        run.call(settings.read().connection().map(|conn| RequestKind::Sync {
            conn,
            account: index,
        }));
    };
    use_hook(sync);
    let close = use_callback(move |()| action.set(None));

    let disabled = busy();
    rsx! {
        h2 { "Account {index}" }
        match state() {
            Some(state) => rsx! {
                Diagnostics { state, account: account.clone() }
            },
            None => rsx! {
                p { "Not synced yet." }
            },
        }
        h3 { "Actions" }
        p {
            button { disabled, onclick: move |_| sync(), "Refresh" }
            for a in Action::ALL {
                " "
                button {
                    disabled: action() == Some(a),
                    onclick: move |_| action.set(Some(a)),
                    "{a.label()}"
                }
            }
        }
        if let Some(a) = action() {
            fieldset {
                legend { "{a.label()}" }
                match a {
                    Action::Deposit => rsx! {
                        DepositForm {
                            account: index,
                            disabled,
                            run,
                            close,
                        }
                    },
                    Action::Transfer => rsx! {
                        TransferForm {
                            account: account.clone(),
                            accounts,
                            disabled,
                            run,
                            close,
                        }
                    },
                    Action::Withdraw => rsx! {
                        WithdrawForm {
                            account: index,
                            disabled,
                            run,
                            close,
                        }
                    },
                }
            }
        }
        if let Some(state) = state() {
            Balances { balances: state.balances }
            Notes { notes: state.notes }
            History { history: state.history, global_history: state.global_history }
        }
    }
}

#[component]
fn Diagnostics(state: AccountState, account: AccountInfo) -> Element {
    rsx! {
        h3 { "Diagnostics" }
        table {
            tbody {
                tr {
                    th { "Receive address" }
                    td { class: "mono", "{account.receiver}" }
                }
                tr {
                    th { "Signer" }
                    td { class: "mono", "{state.signer}" }
                }
                tr {
                    th { "Synced to block" }
                    td { "{state.synced_to}" }
                }
                tr {
                    th { "Tint events" }
                    td { "{state.events}" }
                }
                tr {
                    th { "MMR leaves" }
                    td { "{state.leaves}" }
                }
            }
        }
    }
}

#[component]
fn Balances(balances: Vec<Balance>) -> Element {
    rsx! {
        h3 { "Balances" }
        table {
            thead {
                tr {
                    th { "token" }
                    th { "address" }
                    th { "amount" }
                    th { "notes" }
                }
            }
            tbody {
                for balance in balances {
                    tr {
                        td {
                            TokenSymbol { token: balance.token }
                        }
                        td { class: "mono", "{balance.token}" }
                        td { "{balance.amount}" }
                        td { "{balance.notes}" }
                    }
                }
            }
        }
    }
}

#[component]
fn Notes(notes: Vec<NoteInfo>) -> Element {
    rsx! {
        h3 { "Notes" }
        table {
            thead {
                tr {
                    th { "leaf" }
                    th { "block" }
                    th { "tx" }
                    th { "token" }
                    th { "amount" }
                    th { "spent in block" }
                }
            }
            tbody {
                for note in notes {
                    tr {
                        td { "{note.leaf_index}" }
                        td { "{note.block}" }
                        td { class: "mono", "{note.tx}" }
                        td {
                            TokenSymbol { token: note.token }
                        }
                        td { "{note.amount}" }
                        td {
                            match note.spent_in {
                                Some(block) => rsx! { "{block}" },
                                None => rsx! { "unspent" },
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The account's history, or with the toggle, every Tint transaction.
#[component]
fn History(history: Vec<HistoryEntry>, global_history: Vec<GlobalEntry>) -> Element {
    let mut global = use_signal(|| false);
    rsx! {
        h3 { "History" }
        p {
            label {
                input {
                    r#type: "checkbox",
                    checked: global(),
                    onchange: move |e| global.set(e.checked()),
                }
                " Show every Tint transaction, not just this account's"
            }
        }
        if global() {
            GlobalHistory { global_history, history }
        } else {
            AccountHistory { history }
        }
    }
}

#[component]
fn AccountHistory(history: Vec<HistoryEntry>) -> Element {
    rsx! {
        table {
            thead {
                tr {
                    th { "block" }
                    th { "tx" }
                    th { "kind" }
                    th { "token" }
                    th { "amount" }
                    th { "recipient" }
                }
            }
            tbody {
                for entry in history {
                    tr {
                        td { "{entry.block}" }
                        td { class: "mono", "{entry.tx}" }
                        td { {kind_label(entry.kind)} }
                        td {
                            TokenSymbol { token: entry.token }
                        }
                        td { "{entry.amount}" }
                        td { class: "mono",
                            if let Some(recipient) = entry.recipient {
                                "{recipient}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Every Tint transaction, with what's public about it, marking the ones
/// in the account's `history`.
#[component]
fn GlobalHistory(global_history: Vec<GlobalEntry>, history: Vec<HistoryEntry>) -> Element {
    rsx! {
        table {
            thead {
                tr {
                    th { "block" }
                    th { "tx" }
                    th { "mine" }
                    th { "notes spent" }
                    th { "notes created" }
                    th { "public movements" }
                }
            }
            tbody {
                for entry in global_history {
                    tr {
                        td { "{entry.block}" }
                        td { class: "mono", "{entry.tx}" }
                        td {
                            if history.iter().any(|h| h.tx == entry.tx) {
                                "yes"
                            }
                        }
                        td { "{entry.nullified}" }
                        td { "{entry.committed}" }
                        td {
                            for deposit in entry.deposits {
                                div {
                                    "deposit {deposit.amount} "
                                    TokenSymbol { token: deposit.token }
                                }
                            }
                            for withdrawal in entry.withdrawals {
                                div {
                                    "withdraw {withdrawal.amount} "
                                    TokenSymbol { token: withdrawal.token }
                                    " to "
                                    span { class: "mono", "{withdrawal.recipient}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A token's symbol from the settings, or its address if it's unknown.
#[component]
fn TokenSymbol(token: Address) -> Element {
    let settings = use_context::<Signal<Settings>>();
    let symbol = settings.read().symbol(token);
    match symbol {
        Some(symbol) => rsx! { "{symbol}" },
        None => rsx! {
            span { class: "mono", "{token}" }
        },
    }
}

fn kind_label(kind: HistoryKind) -> &'static str {
    match kind {
        HistoryKind::Deposit => "deposit",
        HistoryKind::Receive => "transfer in",
        HistoryKind::Send => "transfer out",
        HistoryKind::Withdraw => "withdraw",
        HistoryKind::SelfTransfer => "self transfer",
    }
}

async fn run_request(
    client: WorkerClient,
    kind: Result<RequestKind, String>,
    mut state: Signal<Option<AccountState>>,
    mut busy: Signal<bool>,
    mut error: Signal<Option<String>>,
    mut reload: Signal<u64>,
) {
    let kind = match kind {
        Ok(kind) => kind,
        Err(e) => {
            error.set(Some(e));
            return;
        }
    };

    // Anything but a sync may change which accounts are used.
    let changes_accounts = !matches!(kind, RequestKind::Sync { .. });
    busy.set(true);
    let result = client.account_state(kind).await;
    busy.set(false);
    match result {
        Ok(new_state) => {
            state.set(Some(new_state));
            error.set(None);
            if changes_accounts {
                reload += 1;
            }
        }
        Err(e) => error.set(Some(e)),
    }
}

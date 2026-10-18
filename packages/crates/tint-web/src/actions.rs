//! The deposit, transfer, and withdraw forms.
//!
//! Each is its own component, so switching between them starts a fresh form.

use alloy_primitives::Address;
use dioxus::prelude::*;
use tint_web_protocol::{AccountInfo, RequestKind};

use crate::storage::Settings;

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Deposit,
    Transfer,
    Withdraw,
}

impl Action {
    pub const ALL: [Action; 3] = [Action::Deposit, Action::Transfer, Action::Withdraw];

    pub fn label(self) -> &'static str {
        match self {
            Action::Deposit => "Deposit",
            Action::Transfer => "Transfer",
            Action::Withdraw => "Withdraw",
        }
    }
}

#[component]
pub fn DepositForm(
    account: u32,
    disabled: bool,
    run: Callback<Result<RequestKind, String>>,
    close: Callback<()>,
) -> Element {
    let settings = use_context::<Signal<Settings>>();
    let token = use_first_token();
    let amount = use_signal(String::new);

    let request = move || -> Result<RequestKind, String> {
        Ok(RequestKind::Deposit {
            conn: settings.read().connection()?,
            account,
            token: parse_address(&token(), "token")?,
            amount: parse_amount(&amount())?,
        })
    };

    rsx! {
        table {
            tbody {
                TokenRow { token }
                AmountRow { amount }
            }
        }
        p {
            button { disabled, onclick: move |_| run.call(request()), "Submit" }
            " "
            button { onclick: move |_| close.call(()), "Close" }
        }
    }
}

/// Sends to another local account.
#[component]
pub fn TransferForm(
    account: AccountInfo,
    accounts: Signal<Vec<AccountInfo>>,
    disabled: bool,
    run: Callback<Result<RequestKind, String>>,
    close: Callback<()>,
) -> Element {
    let settings = use_context::<Signal<Settings>>();
    let mut to = use_signal(String::new);
    let token = use_first_token();
    let amount = use_signal(String::new);
    let index = account.index;

    let request = move || -> Result<RequestKind, String> {
        let to = to();
        if to.is_empty() {
            return Err("choose an account to transfer to".to_string());
        }
        Ok(RequestKind::Transfer {
            conn: settings.read().connection()?,
            account: index,
            to,
            token: parse_address(&token(), "token")?,
            amount: parse_amount(&amount())?,
        })
    };

    let others: Vec<AccountInfo> = accounts
        .read()
        .iter()
        .filter(|a| a.index != index)
        .cloned()
        .collect();
    rsx! {
        table {
            tbody {
                tr {
                    th { "To account" }
                    td {
                        select {
                            value: "{to}",
                            onchange: move |e| to.set(e.value()),
                            option { value: "", "(choose an account)" }
                            for other in others {
                                option { value: "{other.receiver}", "Account {other.index}" }
                            }
                        }
                    }
                }
                TokenRow { token }
                AmountRow { amount }
            }
        }
        p {
            button { disabled, onclick: move |_| run.call(request()), "Submit" }
            " "
            button { onclick: move |_| close.call(()), "Close" }
        }
    }
}

/// Unshields to a public address.
#[component]
pub fn WithdrawForm(
    account: u32,
    disabled: bool,
    run: Callback<Result<RequestKind, String>>,
    close: Callback<()>,
) -> Element {
    let settings = use_context::<Signal<Settings>>();
    let mut to = use_signal(String::new);
    let token = use_first_token();
    let amount = use_signal(String::new);

    let request = move || -> Result<RequestKind, String> {
        Ok(RequestKind::Withdraw {
            conn: settings.read().connection()?,
            account,
            to: parse_address(&to(), "recipient")?,
            token: parse_address(&token(), "token")?,
            amount: parse_amount(&amount())?,
        })
    };

    rsx! {
        table {
            tbody {
                tr {
                    th { "Recipient address" }
                    td {
                        input {
                            value: "{to}",
                            placeholder: "0x…",
                            oninput: move |e| to.set(e.value()),
                        }
                    }
                }
                TokenRow { token }
                AmountRow { amount }
            }
        }
        p {
            button { disabled, onclick: move |_| run.call(request()), "Submit" }
            " "
            button { onclick: move |_| close.call(()), "Close" }
        }
    }
}

/// Picks one of the tokens listed in the settings.
#[component]
fn TokenRow(token: Signal<String>) -> Element {
    let settings = use_context::<Signal<Settings>>();
    let tokens = settings.read().tokens();
    rsx! {
        tr {
            th { "Token" }
            td {
                select { value: "{token}", onchange: move |e| token.set(e.value()),
                    for t in tokens {
                        option { value: "{t.address}", "{t.symbol} ({t.address})" }
                    }
                }
            }
        }
    }
}

#[component]
fn AmountRow(amount: Signal<String>) -> Element {
    rsx! {
        tr {
            th { "Amount (base units)" }
            td {
                input {
                    value: "{amount}",
                    oninput: move |e| amount.set(e.value()),
                }
            }
        }
    }
}

/// A token selection, starting at the first known token.
fn use_first_token() -> Signal<String> {
    let settings = use_context::<Signal<Settings>>();
    use_signal(move || {
        settings
            .read()
            .tokens()
            .first()
            .map(|t| t.address.to_string())
            .unwrap_or_default()
    })
}

fn parse_address(value: &str, what: &str) -> Result<Address, String> {
    value
        .trim()
        .parse()
        .map_err(|_| format!("invalid {what} address"))
}

fn parse_amount(value: &str) -> Result<u128, String> {
    value
        .trim()
        .parse()
        .map_err(|_| "amount must be a whole number of base units".to_string())
}

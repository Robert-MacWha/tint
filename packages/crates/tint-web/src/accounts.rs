use dioxus::prelude::*;
use tint_web_protocol::AccountInfo;

/// The signer's public account, or one of its private accounts by index.
#[derive(Clone, Copy, PartialEq)]
pub enum Selected {
    Public,
    Account(u32),
}

/// Lists the signer's public account and recovered private accounts,
/// selecting one on click.
#[component]
pub fn Accounts(
    accounts: Signal<Vec<AccountInfo>>,
    selected: Signal<Selected>,
    min: Signal<u32>,
) -> Element {
    let count = accounts.read().len();
    let public_selected = selected() == Selected::Public;
    rsx! {
        h3 { "Accounts" }
        if count == 0 {
            p { "Set a private key to recover accounts." }
        }
        ul {
            li {
                button {
                    disabled: public_selected,
                    onclick: move |_| selected.set(Selected::Public),
                    "Public"
                }
                if public_selected {
                    " (selected)"
                }
            }
            for account in accounts() {
                AccountItem { key: "{account.index}", account, selected }
            }
        }
        button { disabled: count == 0, onclick: move |_| min.set(count as u32 + 1), "Add account" }
    }
}

#[component]
fn AccountItem(account: AccountInfo, selected: Signal<Selected>) -> Element {
    let index = account.index;
    let is_selected = selected() == Selected::Account(index);
    rsx! {
        li {
            button {
                disabled: is_selected,
                onclick: move |_| selected.set(Selected::Account(index)),
                "Account {index}"
            }
            match account.notes {
                0 => rsx! { " unused" },
                1 => rsx! { " 1 note" },
                notes => rsx! { " {notes} notes" },
            }
            if is_selected {
                " (selected)"
            }
        }
    }
}

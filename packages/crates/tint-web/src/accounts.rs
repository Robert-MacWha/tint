use dioxus::prelude::*;
use tint_web_protocol::AccountInfo;

/// Lists the private key's recovered accounts, selecting one on click.
#[component]
pub fn Accounts(
    accounts: Signal<Vec<AccountInfo>>,
    selected: Signal<Option<u32>>,
    min: Signal<u32>,
) -> Element {
    let count = accounts.read().len();
    rsx! {
        h3 { "Accounts" }
        if count == 0 {
            p { "Set the RPC URL, Tint address, and private key to recover accounts." }
        }
        ul {
            for account in accounts() {
                AccountItem { key: "{account.index}", account, selected }
            }
        }
        button { disabled: count == 0, onclick: move |_| min.set(count as u32 + 1), "Add account" }
    }
}

#[component]
fn AccountItem(account: AccountInfo, selected: Signal<Option<u32>>) -> Element {
    let index = account.index;
    let is_selected = selected() == Some(index);
    rsx! {
        li {
            button {
                disabled: is_selected,
                onclick: move |_| selected.set(Some(index)),
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

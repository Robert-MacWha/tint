//! Derives Tint accounts from a signature by the Ethereum private key, so
//! they can be recovered from the key alone.

use alloy::{
    primitives::{B256, keccak256},
    signers::{SignerSync, local::PrivateKeySigner},
};
use tint::{account::Account, policy::Policy, syncer::Snapshot};
use tint_web_protocol::AccountInfo;

use crate::account_state::receiver;

/// The message whose signature roots the accounts. Changing it changes every
/// derived account.
const MESSAGE: &str = "Tint account derivation\n\n\
    Signing this derives your Tint accounts. Only sign it on sites you trust.";

/// Unused accounts scanned past the last used one before recovery stops.
const GAP: u32 = 10;

/// The root every account of `signer` is derived from.
///
/// Local signatures are deterministic (RFC 6979), so this is too.
pub fn account_root(signer: &PrivateKeySigner) -> anyhow::Result<B256> {
    let signature = signer.sign_message_sync(MESSAGE.as_bytes())?;
    Ok(keccak256(signature.as_bytes()))
}

/// The account at `index` under `root`.
pub fn derive_account(root: B256, index: u32) -> Account {
    let mut input = root.to_vec();
    input.extend_from_slice(&index.to_be_bytes());
    Account::from_seed(&keccak256(input).0, Policy::NONE)
}

/// The accounts under `root` up to the last one with notes in `snapshot`,
/// and at least `min` of them.
pub fn recover_accounts(snapshot: &Snapshot, root: B256, min: u32) -> Vec<AccountInfo> {
    let mut accounts = Vec::new();
    let mut used = 0;
    let mut index = 0;
    while index < min.max(used + GAP) {
        let account = derive_account(root, index);
        let notes = snapshot.unspent(&account).len() + snapshot.spent(&account).len();
        if notes > 0 {
            used = index + 1;
        }
        accounts.push(AccountInfo {
            index,
            receiver: receiver(&account),
            notes,
        });
        index += 1;
    }
    accounts.truncate(min.max(used) as usize);
    accounts
}

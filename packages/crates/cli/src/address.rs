use anyhow::Context;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use tint::account::pub_account::PubAccount;

use crate::config;

const PREFIX: &str = "tint:";

/// Encodes a receiving address as `tint:` followed by base64url.
pub fn encode(pub_account: &PubAccount) -> String {
    format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(pub_account.to_bytes()))
}

/// Resolves a recipient given as an encoded address or a local account name.
pub fn parse_recipient(recipient: &str) -> anyhow::Result<PubAccount> {
    let Some(encoded) = recipient.strip_prefix(PREFIX) else {
        return Ok(config::load_account(recipient)?.account().pub_account());
    };
    decode(encoded)
}

fn decode(encoded: &str) -> anyhow::Result<PubAccount> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded.trim())
        .context("address is not valid base64url")?;
    PubAccount::from_bytes(&bytes).context("invalid tint address")
}

#[cfg(test)]
mod tests {
    use tint::{account::Account, policy::Policy};

    use super::*;

    #[test]
    fn encoded_address_parses_as_recipient() {
        let pub_account = Account::from_seed(&[7; 32], Policy::NONE).pub_account();

        let parsed = parse_recipient(&encode(&pub_account)).unwrap();

        assert_eq!(parsed, pub_account);
    }

    #[test]
    fn truncated_address_is_rejected() {
        let pub_account = Account::from_seed(&[7; 32], Policy::NONE).pub_account();
        let encoded = encode(&pub_account);

        assert!(parse_recipient(&encoded[..encoded.len() - 4]).is_err());
    }
}

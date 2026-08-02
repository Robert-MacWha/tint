use std::io::{self, Write};

use alloy::{hex, primitives::Address};
use k256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashSigner};
use tint::account::{Account, keys::Keys};
use tint_multisig_spendability::{
    N_SIGNERS, THRESHOLD,
    account::{MultisigSpendingAccount, Signer},
    ffi::artifacts as multisig_artifacts,
};

use crate::{account::resolve_spendability_address, config::SpendabilityState};

const SPENDABILITY_ADDRESS_ENV_VAR: &str = "MULTISIG_SPENDABILITY_ADDRESS";

/// A multisig signer who prompts the user to sign the operation hash and return
/// the resulting signature.
struct PromptSigner {
    index: usize,
    verifying_key: VerifyingKey,
}

pub fn create_multisig_account(
    name: &str,
    keys: Keys,
) -> anyhow::Result<(Account, SpendabilityState)> {
    let address = resolve_spendability_address(SPENDABILITY_ADDRESS_ENV_VAR, name)?;
    let pub_keys = prompt_multisig_pub_keys(name)?;
    let account = multisig_account(keys, address, &pub_keys)?;
    let signers = pub_keys.map(|pk| hex::encode_prefixed(pk.to_sec1_bytes()));
    Ok((account, SpendabilityState::Multisig { address, signers }))
}

pub fn load_multisig_account(
    keys: Keys,
    address: Address,
    signers: &[String; N_SIGNERS],
) -> anyhow::Result<Account> {
    let pub_keys = tint::array::try_from_fn(|i| -> anyhow::Result<VerifyingKey> {
        Ok(VerifyingKey::from_sec1_bytes(&hex::decode(&signers[i])?)?)
    })?;
    multisig_account(keys, address, &pub_keys)
}

fn multisig_account(
    keys: Keys,
    contract_address: Address,
    pub_keys: &[VerifyingKey; N_SIGNERS],
) -> anyhow::Result<Account> {
    let signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS] = std::array::from_fn(|i| {
        Box::new(PromptSigner {
            index: i,
            verifying_key: pub_keys[i],
        }) as Box<dyn Signer + Send + Sync>
    });
    let account = MultisigSpendingAccount::<N_SIGNERS, THRESHOLD>::new(
        contract_address,
        signers,
        multisig_artifacts::ccs_bytes()?,
        multisig_artifacts::proving_key_bytes()?,
        multisig_artifacts::verifying_key_bytes()?,
    )?;
    Ok(Account::from_keys(keys, account))
}

/// Prompts once per signer for their uncompressed secp256k1 public key.
fn prompt_multisig_pub_keys(name: &str) -> anyhow::Result<[VerifyingKey; N_SIGNERS]> {
    tint::array::try_from_fn(|i| -> anyhow::Result<VerifyingKey> {
        print!(
            "Public key for signer {} of {N_SIGNERS} on account \"{name}\": ",
            i + 1
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        parse_pubkey(input.trim())
    })
}

/// Parses a secp256k1 public key, accepting either full SEC1 encoding
/// (leading tag byte) or a bare 64-byte `X||Y`.
fn parse_pubkey(input: &str) -> anyhow::Result<VerifyingKey> {
    let bytes = hex::decode(input)?;
    let sec1_bytes = match bytes.len() {
        64 => [&[0x04], bytes.as_slice()].concat(),
        _ => bytes,
    };
    Ok(VerifyingKey::from_sec1_bytes(&sec1_bytes)?)
}

impl PrehashSigner<Signature> for PromptSigner {
    fn sign_prehash(&self, prehash: &[u8]) -> Result<Signature, signature::Error> {
        self.sign_prehash_optional(prehash)?
            .ok_or_else(|| signature::Error::from_source("signature required but not provided"))
    }
}

impl Signer for PromptSigner {
    fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key
    }

    fn sign_prehash_optional(&self, prehash: &[u8]) -> Result<Option<Signature>, signature::Error> {
        prompt_signature(self.index, &self.verifying_key, prehash)
            .map_err(signature::Error::from_source)
    }
}

/// Prompts for a signature, returning `None` if the user leaves the input
/// blank (e.g. because enough other signers already meet the threshold).
fn prompt_signature(
    index: usize,
    verifying_key: &VerifyingKey,
    prehash: &[u8],
) -> Result<Option<Signature>, Box<dyn std::error::Error + Send + Sync>> {
    let hash_hex = hex::encode_prefixed(prehash);
    let pubkey_hex = hex::encode_prefixed(&verifying_key.to_sec1_point(false).as_bytes()[1..]);
    let signer_number = index + 1;
    println!(
        "Signer {signer_number} (pubkey {pubkey_hex}) needs to sign the operation hash:\n  {hash_hex}\ne.g. `cast wallet sign --no-hash {hash_hex} --private-key <key>`"
    );
    print!("Paste the resulting signature, or leave blank to skip this signer: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        println!("Skipping signer {signer_number}");
        return Ok(None);
    }

    let bytes = hex::decode(input)?;
    let sig_bytes: &[u8] = match bytes.len() {
        64 => &bytes[..],
        65 => &bytes[..64],
        n => return Err(format!("expected a 64 or 65 byte signature, got {n} bytes").into()),
    };
    Ok(Some(Signature::from_slice(sig_bytes)?))
}

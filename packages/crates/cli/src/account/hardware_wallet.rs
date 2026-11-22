use alloy::primitives::{Address, B256};
use alloy_signer_local::PrivateKeySigner;
use tint::{operation::Operation, policy::Policy};
use tint_multisig_policy::{
    ffi::artifacts::load_hardware_wallet_artifacts, hardware_wallet::HardwareWalletPolicy,
};
use tracing::info;

use crate::config::{HardwareSigner, SpendPolicy};

/// A hardware-wallet policy, signing with a Ledger or a local key.
pub enum HardwareWallet {
    Local(HardwareWalletPolicy<PrivateKeySigner>),
    #[cfg(feature = "ledger")]
    Ledger(HardwareWalletPolicy<alloy_signer_ledger::LedgerSigner>),
}

impl HardwareWallet {
    /// Connects to `signer`, which signs a setup message to share its public key.
    pub async fn load(
        address: Address,
        chain_id: u64,
        signer: &HardwareSigner,
    ) -> anyhow::Result<Self> {
        let artifacts = load_hardware_wallet_artifacts()?;
        match signer {
            HardwareSigner::PrivateKey(key) => {
                let signer = PrivateKeySigner::from_bytes(key)?;
                let policy =
                    HardwareWalletPolicy::new(address, chain_id, signer, artifacts).await?;
                Ok(Self::Local(policy))
            }
            HardwareSigner::Ledger { index } => ledger(address, chain_id, *index, artifacts).await,
        }
    }

    pub fn policy(&self) -> Policy {
        match self {
            Self::Local(policy) => policy.policy(),
            #[cfg(feature = "ledger")]
            Self::Ledger(policy) => policy.policy(),
        }
    }

    pub async fn authorize(&self, op: &mut Operation) -> anyhow::Result<()> {
        info!("Sign the operation on your signer");
        match self {
            Self::Local(policy) => policy.authorize(op).await?,
            #[cfg(feature = "ledger")]
            Self::Ledger(policy) => policy.authorize(op).await?,
        }
        Ok(())
    }
}

/// Connects to a new account's signer, returning its policy.
pub async fn create(
    address: Address,
    ledger_index: Option<usize>,
    signer_key: Option<B256>,
) -> anyhow::Result<(Policy, SpendPolicy)> {
    let signer = match (ledger_index, signer_key) {
        (_, Some(key)) => HardwareSigner::PrivateKey(key),
        (Some(index), None) => HardwareSigner::Ledger { index },
        (None, None) => anyhow::bail!("pass --ledger-index or --signer-key"),
    };

    //? The chain id only affects spend signatures, not the policy.
    let policy = HardwareWallet::load(address, 0, &signer).await?.policy();
    Ok((policy, SpendPolicy::HardwareWallet(signer)))
}

#[cfg(feature = "ledger")]
async fn ledger(
    address: Address,
    chain_id: u64,
    index: usize,
    artifacts: tint_multisig_policy::ffi::artifacts::Artifacts,
) -> anyhow::Result<HardwareWallet> {
    info!("Confirm the setup message on your Ledger");
    let policy = HardwareWalletPolicy::ledger(address, chain_id, index, artifacts).await?;
    Ok(HardwareWallet::Ledger(policy))
}

#[cfg(not(feature = "ledger"))]
#[expect(
    clippy::unused_async,
    reason = "mirrors the `ledger` feature's signature"
)]
async fn ledger(
    _address: Address,
    _chain_id: u64,
    _index: usize,
    _artifacts: tint_multisig_policy::ffi::artifacts::Artifacts,
) -> anyhow::Result<HardwareWallet> {
    anyhow::bail!("Ledger accounts need tint-cli built with `--features ledger`")
}

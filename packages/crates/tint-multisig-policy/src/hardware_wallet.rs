use alloy_primitives::{Address, B256, Bytes, eip191_hash_message};
use alloy_signer::Signer;
use alloy_sol_types::{Eip712Domain, SolStruct, eip712_domain, sol};
use ark_bn254::Fr;
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use tint::{
    fr::{address_to_fr, fr_to_b256},
    operation::{Operation, OperationError},
    policy::Policy,
};

use crate::{
    ffi::{self, artifacts::Artifacts},
    pubkey_hash::{self, InvalidPublicKey},
};

sol! {
    /// The EIP-712 message a signer signs to authorize an operation.
    ///
    /// Mirrors `HardwareWalletPolicy.OPERATION_TYPEHASH`.
    struct TintOperation {
        bytes32 signingHash;
    }
}

/// The EIP-191 message signed once to recover a signer's public key.
pub const SETUP_MESSAGE: &str =
    "Tint: share this account's public key with the hardware wallet policy";

/// Authorizes spending notes bound to a single secp256k1 signer, such as a
/// hardware wallet, via the Go/gnark circuit in `go/circuit/hardwarewallet.go`.
///
/// The signer signs the EIP-712 [`TintOperation`] message over the
/// operation's signing hash. Any [`alloy_signer::Signer`] works, including
/// `alloy-signer-ledger` and `alloy-signer-trezor`.
pub struct HardwareWalletPolicy<S> {
    contract_address: Address,
    chain_id: u64,
    signer: S,
    pub_key: VerifyingKey,
    /// `pubkey_hash::pubkey_hash(&[pub_key])`.
    witness: Fr,
    artifacts: Artifacts,
}

#[derive(Debug, thiserror::Error)]
pub enum HardwareWalletPolicyError {
    #[error("operation error: {0}")]
    Operation(#[from] OperationError),
    #[error("signer error: {0}")]
    Signer(#[from] alloy_signer::Error),
    #[error("invalid signature: {0}")]
    Signature(#[from] k256::ecdsa::Error),
    #[error(transparent)]
    InvalidPublicKey(#[from] InvalidPublicKey),
    #[error("recovered public key does not match the signer's address")]
    WrongPublicKey,
    #[error("proving error: {0}")]
    Ffi(#[from] ffi::FfiError),
    #[cfg(feature = "ledger")]
    #[error("ledger error: {0}")]
    Ledger(#[from] alloy_signer_ledger::LedgerError),
}

impl<S: Signer + Send + Sync> HardwareWalletPolicy<S> {
    /// Binds a policy to `signer`, asking it to sign [`SETUP_MESSAGE`].
    ///
    /// The signature recovers the signer's public key, which the policy
    /// commits to. `chain_id` and `contract_address` form the EIP-712 domain.
    pub async fn new(
        contract_address: Address,
        chain_id: u64,
        signer: S,
        artifacts: Artifacts,
    ) -> Result<Self, HardwareWalletPolicyError> {
        let signature = signer.sign_message(SETUP_MESSAGE.as_bytes()).await?;
        let prehash = eip191_hash_message(SETUP_MESSAGE);
        let pub_key = recover(&signature, prehash)?;
        if address_of(&pub_key)? != signer.address() {
            return Err(HardwareWalletPolicyError::WrongPublicKey);
        }

        let witness = pubkey_hash::pubkey_hash(&[pub_key])?;
        Ok(Self {
            contract_address,
            chain_id,
            signer,
            pub_key,
            witness,
            artifacts,
        })
    }

    /// The policy notes for this signer are bound to.
    #[must_use]
    pub fn policy(&self) -> Policy {
        Policy::new(self.contract_address, self.witness)
    }

    /// Authorizes every input of `op` bound to this policy.
    ///
    /// # Errors
    ///
    /// Errors if signing or proving fails, or no input of `op` is bound to
    /// this policy.
    pub async fn authorize(&self, op: &mut Operation) -> Result<(), HardwareWalletPolicyError> {
        let input = self.prove(op).await?;
        op.authorize(self.policy(), input)?;
        Ok(())
    }

    /// Asks the signer to sign `op` and proves the signature is valid.
    ///
    /// The returned policy input authorizes every input of `op` bound to this
    /// policy.
    ///
    /// # Errors
    ///
    /// Errors if the signer fails or proving fails.
    pub async fn prove(&self, op: &Operation) -> Result<Bytes, HardwareWalletPolicyError> {
        let witness = op.witness();
        let message = TintOperation {
            signingHash: fr_to_b256(witness.signing_hash()),
        };
        let domain = self.domain();
        let signature = self.signer.sign_typed_data(&message, &domain).await?;
        let signature = to_k256(&signature)?;

        let solidity_proof = ffi::prove_hardware_wallet_via_go(
            &self.artifacts,
            address_to_fr(self.contract_address),
            witness,
            witness.context_hash(),
            message.eip712_signing_hash(&domain),
            &self.pub_key,
            &signature,
        )?;

        Ok(Bytes::from(solidity_proof))
    }

    /// The EIP-712 domain of the policy contract.
    ///
    /// Mirrors `HardwareWalletPolicy`'s `EIP712` constructor.
    fn domain(&self) -> Eip712Domain {
        eip712_domain! {
            name: "Tint Hardware Wallet Policy",
            version: "1",
            chain_id: self.chain_id,
            verifying_contract: self.contract_address,
        }
    }
}

#[cfg(feature = "ledger")]
impl HardwareWalletPolicy<alloy_signer_ledger::LedgerSigner> {
    /// Binds a policy to the Ledger Live account at `index` on a USB Ledger.
    pub async fn ledger(
        contract_address: Address,
        chain_id: u64,
        index: usize,
        artifacts: Artifacts,
    ) -> Result<Self, HardwareWalletPolicyError> {
        let path = alloy_signer_ledger::HDPath::LedgerLive(index);
        let signer = alloy_signer_ledger::LedgerSigner::new(path, Some(chain_id)).await?;
        Self::new(contract_address, chain_id, signer, artifacts).await
    }
}

impl<S> std::fmt::Debug for HardwareWalletPolicy<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HardwareWalletPolicy")
            .field("contract_address", &self.contract_address)
            .field("chain_id", &self.chain_id)
            .finish_non_exhaustive()
    }
}

fn recover(
    signature: &alloy_primitives::Signature,
    prehash: B256,
) -> Result<VerifyingKey, HardwareWalletPolicyError> {
    let rs = to_k256(signature)?;
    let recovery_id = RecoveryId::new(signature.v(), false);
    Ok(VerifyingKey::recover_from_prehash(
        prehash.as_slice(),
        &rs,
        recovery_id,
    )?)
}

fn address_of(pub_key: &VerifyingKey) -> Result<Address, InvalidPublicKey> {
    let (x, y) = pubkey_hash::encoded_xy(pub_key)?;
    Ok(Address::from_raw_public_key(&[x, y].concat()))
}

fn to_k256(signature: &alloy_primitives::Signature) -> Result<Signature, k256::ecdsa::Error> {
    Signature::from_scalars(signature.r().to_be_bytes(), signature.s().to_be_bytes())
}

#[cfg(test)]
mod tests {
    use alloy_signer_local::PrivateKeySigner;
    use k256::ecdsa::SigningKey;

    use super::*;

    #[tokio::test]
    async fn new_commits_to_signer_public_key() {
        let signer = PrivateKeySigner::random();
        let key = SigningKey::from_slice(signer.to_bytes().as_slice()).unwrap();
        let artifacts = Artifacts {
            ccs: vec![],
            pk: vec![],
            vk: vec![],
        };

        let policy = HardwareWalletPolicy::new(Address::ZERO, 1, signer, artifacts)
            .await
            .unwrap();

        let witness = pubkey_hash::pubkey_hash(&[*key.verifying_key()]).unwrap();
        assert_eq!(policy.policy(), Policy::new(Address::ZERO, witness));
    }
}

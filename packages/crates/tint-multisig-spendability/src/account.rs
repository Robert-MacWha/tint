use alloy_primitives::{Address, Bytes};
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use k256::ecdsa::{Signature, VerifyingKey};

use tint::{
    account::spending::{SpendingAccount, SpendingAccountError},
    circuit::join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
    fr::address_to_fr,
    note::commitment::{NullifiableCommitment, SpendableCommitment},
    operation::Operation,
};

use crate::{N_SIGNERS, THRESHOLD, ffi, pubkey_hash};

/// A signer capable of producing an ECDSA/secp256k1 signature over an
/// already-final digest (no further hashing — the circuit signs
/// `operation_hash`'s raw bytes directly as the scalar, see
/// `go/circuit/multisig.go`'s `operationHashAsScalar`), plus recovering its
/// own public key.
pub trait Signer: signature::hazmat::PrehashSigner<Signature> {
    fn verifying_key(&self) -> VerifyingKey;

    /// Signs `prehash`, or returns `Ok(None)` if this signer chooses not to
    /// participate (e.g. because enough other signers already meet
    /// `THRESHOLD`). A missing signature is encoded on-chain/in-circuit as
    /// `r = s = 0`, which the ECDSA-verify gadget treats as automatically
    /// invalid rather than failing proof generation.
    fn sign_prehash_optional(&self, prehash: &[u8]) -> Result<Option<Signature>, signature::Error> {
        self.sign_prehash(prehash).map(Some)
    }
}

impl Signer for k256::ecdsa::SigningKey {
    fn verifying_key(&self) -> VerifyingKey {
        *self.verifying_key()
    }
}

/// A [`SpendingAccount`] for notes using the [`MultisigSpendability`] rule:
/// an M-of-N secp256k1-ECDSA multisig proven via the Go/gnark circuit in
/// `go/circuit/multisig.go`.
///
/// [`MultisigSpendability`]: https://github.com/Robert-MacWha/tint
pub struct MultisigSpendingAccount<const N_SIGNERS: usize, const THRESHOLD: usize> {
    contract_address: Address,
    signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS],
    /// `pubkey_hash::pubkey_hash(&pub_keys)`, computed once at construction
    /// so `spendability_witness()` (whose signature is fixed by
    /// [`SpendingAccount`]) can return it infallibly.
    witness: Fr,
    ccs: Vec<u8>,
    pk: Vec<u8>,
    vk: Vec<u8>,
}

impl<const N_SIGNERS: usize, const THRESHOLD: usize> MultisigSpendingAccount<N_SIGNERS, THRESHOLD> {
    pub fn new(
        contract_address: Address,
        signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS],
        ccs: Vec<u8>,
        pk: Vec<u8>,
        vk: Vec<u8>,
    ) -> Result<Self, pubkey_hash::InvalidPublicKey> {
        debug_assert!(THRESHOLD <= N_SIGNERS);
        let pub_keys: [VerifyingKey; N_SIGNERS] =
            std::array::from_fn(|i| signers[i].verifying_key());
        let witness = pubkey_hash::pubkey_hash(&pub_keys)?;
        Ok(Self {
            contract_address,
            signers,
            witness,
            ccs,
            pk,
            vk,
        })
    }

    fn pub_keys(&self) -> [VerifyingKey; N_SIGNERS] {
        std::array::from_fn(|i| self.signers[i].verifying_key())
    }
}

impl<const N_SIGNERS: usize, const THRESHOLD: usize> std::fmt::Debug
    for MultisigSpendingAccount<N_SIGNERS, THRESHOLD>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultisigSpendingAccount")
            .field("contract_address", &self.contract_address)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl SpendingAccount for MultisigSpendingAccount<N_SIGNERS, THRESHOLD> {
    fn spendability_address(&self) -> Address {
        self.contract_address
    }

    fn spendability_witness(&self) -> Fr {
        self.witness
    }

    /// Proves that at least `THRESHOLD` of this account's `N_SIGNERS` signers
    /// validly signed the operation.
    async fn into_spendable(
        &self,
        base: NullifiableCommitment,
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    ) -> Result<SpendableCommitment, SpendingAccountError> {
        let operation_hash = operation.hash();
        let msg = fr_to_be_bytes(operation_hash);

        let pub_keys = self.pub_keys();
        let signatures: [Option<Signature>; N_SIGNERS] =
            tint::array::try_from_fn(|i| self.signers[i].sign_prehash_optional(&msg))
                .map_err(SpendingAccountError::new)?;

        let solidity_proof = ffi::prove_via_go(
            &self.ccs,
            &self.pk,
            &self.vk,
            address_to_fr(self.contract_address),
            &operation,
            &pub_keys,
            &signatures,
        )
        .map_err(SpendingAccountError::new)?;

        Ok(SpendableCommitment::new(
            base.inner.asset,
            base.inner.amount,
            base.nullifier_key,
            self.spendability_address(),
            self.spendability_witness(),
            Bytes::from(solidity_proof),
            base.inner.random,
        ))
    }
}

fn fr_to_be_bytes(f: Fr) -> [u8; 32] {
    let be = f.into_bigint().to_bytes_be();
    let mut out = [0u8; 32];
    out[32 - be.len()..].copy_from_slice(&be);
    out
}

#[cfg(test)]
mod tests {
    use k256::ecdsa::SigningKey;
    use k256::elliptic_curve::Generate;
    use tint::{account::keys::NullifierKey, note::commitment::BaseCommitment};

    use super::*;

    #[tokio::test]
    #[ignore = "run with `cargo test --release -- --ignored`"]
    async fn multisig() {
        let contract_address = Address::new([9u8; 20]);

        let signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS] = std::array::from_fn(|_| {
            Box::new(SigningKey::generate()) as Box<dyn Signer + Send + Sync>
        });

        let account = MultisigSpendingAccount::<N_SIGNERS, THRESHOLD>::new(
            contract_address,
            signers,
            ffi::artifacts::ccs_bytes().unwrap(),
            ffi::artifacts::proving_key_bytes().unwrap(),
            ffi::artifacts::verifying_key_bytes().unwrap(),
        )
        .unwrap();

        let mut operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS> = Operation::default();
        operation.inputs[0].spendability_address = account.spendability_address();
        operation.inputs[0].spendability_witness = account.spendability_witness();

        let base = NullifiableCommitment::new(BaseCommitment::default(), NullifierKey::default());

        account.into_spendable(base, operation).await.unwrap();
    }
}

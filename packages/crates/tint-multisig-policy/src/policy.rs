use alloy_primitives::{Address, Bytes, U256};
use ark_bn254::{Bn254, Fr};
use ark_ff::{BigInteger, PrimeField};
use k256::ecdsa::{Signature, VerifyingKey};
use tint::{
    abis::tint::ProofLib,
    fr::address_to_fr,
    operation::UnauthorizedOperation,
    policy::{Authorizer, Policy},
};
use tint_groth16::prove::placeholder;

use crate::{
    N_SIGNERS, THRESHOLD,
    ffi::{self, artifacts::Artifacts},
    pubkey_hash,
};

/// Length of the proof `MultisigVerifier.verifyProof` takes.
const PROOF_LEN: usize = 384;

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

/// Authorizes spending notes bound to the [`MultisigPolicyCircuit`] rule: an
/// M-of-N secp256k1-ECDSA multisig proven via the Go/gnark circuit in
/// `go/circuit/multisig.go`.
///
/// [`MultisigPolicyCircuit`]: https://github.com/Robert-MacWha/tint
pub struct MultisigPolicy<const N_SIGNERS: usize, const THRESHOLD: usize> {
    contract_address: Address,
    signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS],
    /// `pubkey_hash::pubkey_hash(&pub_keys)`, computed once at construction
    /// so [`Self::policy`] can return it infallibly.
    witness: Fr,
    artifacts: Artifacts,
}

#[derive(Debug, thiserror::Error)]
pub enum MultisigPolicyError {
    #[error("signing error: {0}")]
    Signature(#[from] signature::Error),
    #[error("proving error: {0}")]
    Ffi(#[from] ffi::FfiError),
}

impl<const N_SIGNERS: usize, const THRESHOLD: usize> MultisigPolicy<N_SIGNERS, THRESHOLD> {
    pub fn new(
        contract_address: Address,
        signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS],
        artifacts: Artifacts,
    ) -> Result<Self, pubkey_hash::InvalidPublicKey> {
        const {
            assert!(THRESHOLD <= N_SIGNERS, "THRESHOLD must be <= N_SIGNERS");
        }

        let pub_keys: [VerifyingKey; N_SIGNERS] =
            std::array::from_fn(|i| signers[i].verifying_key());
        let witness = pubkey_hash::pubkey_hash(&pub_keys)?;

        Ok(Self {
            contract_address,
            signers,
            witness,
            artifacts,
        })
    }

    fn pub_keys(&self) -> [VerifyingKey; N_SIGNERS] {
        std::array::from_fn(|i| self.signers[i].verifying_key())
    }
}

impl<const N_SIGNERS: usize, const THRESHOLD: usize> std::fmt::Debug
    for MultisigPolicy<N_SIGNERS, THRESHOLD>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultisigPolicy")
            .field("contract_address", &self.contract_address)
            .finish_non_exhaustive()
    }
}

impl MultisigPolicy<N_SIGNERS, THRESHOLD> {
    /// The policy notes for this multisig are bound to.
    #[must_use]
    pub fn policy(&self) -> Policy {
        Policy::new(self.contract_address, self.witness)
    }

    /// Proves that at least `THRESHOLD` of this multisig's `N_SIGNERS`
    /// signers validly signed `op`'s signing hash. The returned policy input
    /// authorizes every input of `op` bound to this policy.
    ///
    /// # Errors
    ///
    /// Errors if a signer fails or proving fails.
    pub fn prove(&self, op: &UnauthorizedOperation) -> Result<Bytes, MultisigPolicyError> {
        let operation = op.operation();
        let msg = fr_to_be_bytes(operation.signing_hash());

        let pub_keys = self.pub_keys();
        let signatures: [Option<Signature>; N_SIGNERS] =
            tint::array::try_from_fn(|i| self.signers[i].sign_prehash_optional(&msg))?;

        let solidity_proof = ffi::prove_via_go(
            &self.artifacts,
            address_to_fr(self.contract_address),
            operation,
            operation.context_hash(),
            &pub_keys,
            &signatures,
        )?;

        Ok(Bytes::from(solidity_proof))
    }
}

impl Authorizer for MultisigPolicy<N_SIGNERS, THRESHOLD> {
    type Error = MultisigPolicyError;

    fn placeholder(&self, op: &UnauthorizedOperation) -> Vec<Bytes> {
        op.policy_inputs(self.contract_address, &placeholder_proof())
    }

    fn authorize(&self, op: &UnauthorizedOperation) -> Result<Vec<Bytes>, Self::Error> {
        Ok(op.policy_inputs(self.contract_address, &self.prove(op)?))
    }
}

/// A placeholder for the verifier's 384-byte proof: the Groth16 points
/// (A, B, C), then the Pedersen commitment and its proof of knowledge.
///
/// The commitment and its proof are the point at infinity, which passes the
/// verifier's commitment check, so the placeholder fails only at the final
/// pairing and costs as much gas as a real proof.
fn placeholder_proof() -> Bytes {
    let groth16 = ProofLib::Proof::from(placeholder::<Bn254>());
    let mut proof: Vec<u8> = groth16
        .proof
        .iter()
        .flat_map(U256::to_be_bytes::<32>)
        .collect();
    proof.resize(PROOF_LEN, 0);
    proof.into()
}

fn fr_to_be_bytes(f: Fr) -> [u8; 32] {
    let be = f.into_bigint().to_bytes_be();
    let mut out = [0u8; 32];
    out[32 - be.len()..].copy_from_slice(&be);
    out
}

#[cfg(test)]
mod tests {
    use alloy_primitives::B256;
    use ark_std::rand::{SeedableRng, rngs::StdRng};
    use k256::{ecdsa::SigningKey, elliptic_curve::Generate};
    use tint::{
        account::Account,
        note::{asset::AssetId, owned::OwnedNote},
        operation::OperationBuilder,
    };

    use super::*;

    #[test]
    #[ignore = "run with `cargo test --release -- --ignored`"]
    fn multisig() {
        let contract_address = Address::new([9u8; 20]);

        let signers: [Box<dyn Signer + Send + Sync>; N_SIGNERS] = std::array::from_fn(|_| {
            Box::new(SigningKey::generate()) as Box<dyn Signer + Send + Sync>
        });

        let multisig = MultisigPolicy::<N_SIGNERS, THRESHOLD>::new(
            contract_address,
            signers,
            ffi::artifacts::load_artifacts().unwrap(),
        )
        .unwrap();

        // Spend a note bound to this multisig back to its own account.
        let account = Account::from_seed(&[1u8; 32], multisig.policy());
        let note = account
            .pub_account()
            .commitment(AssetId::default(), 100, B256::ZERO);
        let op = OperationBuilder::new()
            .spend(OwnedNote::new(note, &account).unwrap())
            .change_to(account.pub_account())
            .build(&mut StdRng::seed_from_u64(1))
            .unwrap();

        multisig.prove(&op).unwrap();
    }
}

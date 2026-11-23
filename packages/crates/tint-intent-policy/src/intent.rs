use alloy_primitives::{Address, B256};
use ark_bn254::Fr;
use ark_std::{UniformRand, rand::Rng};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use tint::{
    account::{Account, keys::NullifierKey, pub_account::PubAccount},
    fr::fr_to_b256,
    note::{asset::AssetId, commitment::Commitment, owned::OwnedNote},
    operation::OperationBuilder,
    policy::Policy,
    syncer::{Snapshot, SyncEvent},
};
use tint_groth16::serde::field::FieldAsBytes;
use tint_poseidon2::poseidon2_compress;

/// Extends [`OperationBuilder`] with intent fills.
pub trait OperationBuilderIntentExt {
    /// Spends `intent`'s offered note and creates the note it wants.
    #[must_use]
    fn fill(self, intent: &Intent) -> Self;
}

/// A shielded offer: the `offer` note, spendable by anyone who creates `want`.
///
/// This is the standard intent format makers and relays exchange.
#[serde_with::serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    /// The `IntentPolicy` deployment `offer` is bound to.
    pub policy_address: Address,
    /// The offered note.
    pub offer: Commitment,
    /// `offer`'s one-time nullifying key.
    #[serde_as(as = "FieldAsBytes")]
    pub nullifier_key: Fr,
    /// The exact note a fill must create.
    pub want: Commitment,
    /// Unix time after which fills are refused.
    pub expiry: u64,
    /// `poseidon2(poseidon2(cancelKey))`.
    #[serde_as(as = "FieldAsBytes")]
    pub cancel_hash: Fr,
}

/// The owner's secret for cancelling an [`Intent`].
#[derive(Clone, Copy)]
pub struct CancelKey(pub Fr);

#[derive(Debug, thiserror::Error)]
pub enum IntentError {
    #[error("offer and want amounts must be non-zero")]
    ZeroAmount,
    #[error("offer is not bound to the intent's terms")]
    OfferMismatch,
}

impl Intent {
    /// Creates an intent offering `offer` for exactly `want`, paid to `receiver`.
    ///
    /// The owner funds it by creating [`Self::offer`] (e.g. with
    /// [`OperationBuilder::output_note`]) and keeps the returned key to cancel.
    ///
    /// # Errors
    ///
    /// Errors if either amount is zero.
    pub fn new<R: RngCore + CryptoRng>(
        policy_address: Address,
        offer: (AssetId, u128),
        want: (AssetId, u128),
        receiver: &PubAccount,
        expiry: u64,
        rng: &mut R,
    ) -> Result<(Self, CancelKey), IntentError> {
        if offer.1 == 0 || want.1 == 0 {
            return Err(IntentError::ZeroAmount);
        }

        let cancel_key = CancelKey(Fr::rand(rng));
        let nullifier_key = NullifierKey(Fr::rand(rng));
        let want_note = receiver.commitment(want.0, want.1, B256::new(rng.r#gen()));

        let mut intent = Self {
            policy_address,
            offer: Commitment::default(),
            nullifier_key: nullifier_key.0,
            want: want_note,
            expiry,
            cancel_hash: cancel_key.hash(),
        };
        intent.offer = Commitment::new(
            offer.0,
            offer.1,
            intent.policy().hash(),
            nullifier_key.pub_key(),
            B256::new(rng.r#gen()),
        );
        Ok((intent, cancel_key))
    }

    /// The intent's identifier: its offer's commitment.
    #[must_use]
    pub fn id(&self) -> B256 {
        fr_to_b256(self.offer.hash())
    }

    /// The policy [`Self::offer`] is bound to.
    #[must_use]
    pub fn policy(&self) -> Policy {
        Policy::new(self.policy_address, self.policy_witness())
    }

    /// `poseidon2(wantHash, expiry, cancelHash)`.
    #[must_use]
    pub fn policy_witness(&self) -> Fr {
        poseidon2_compress(&[self.want.hash(), Fr::from(self.expiry), self.cancel_hash])
    }

    /// [`Self::offer`] as a spendable note.
    #[must_use]
    pub fn note(&self) -> OwnedNote {
        OwnedNote {
            note: self.offer,
            nullifier_key: NullifierKey(self.nullifier_key),
            policy: self.policy(),
        }
    }

    /// [`Self::want`] as `owner`'s note, once `snapshot` shows a fill created it.
    ///
    /// Fills don't encrypt the wanted note, so syncing alone doesn't find it.
    #[must_use]
    pub fn received(&self, snapshot: &Snapshot, owner: &Account) -> Option<OwnedNote> {
        let want = self.want.hash();
        let filled = snapshot
            .events
            .iter()
            .filter_map(SyncEvent::commitment)
            .any(|(_, commitment)| commitment == want);
        if !filled {
            return None;
        }
        OwnedNote::new(self.want, owner).ok()
    }

    /// Checks the offer is spendable under the intent's terms.
    ///
    /// # Errors
    ///
    /// Errors if an amount is zero, or the offer isn't bound to these terms.
    pub fn validate(&self) -> Result<(), IntentError> {
        if self.offer.amount == 0 || self.want.amount == 0 {
            return Err(IntentError::ZeroAmount);
        }
        if self.offer.nullifier_pub_key != NullifierKey(self.nullifier_key).pub_key()
            || self.offer.policy_hash != self.policy().hash()
        {
            return Err(IntentError::OfferMismatch);
        }
        Ok(())
    }

    /// Whether `token` is this intent's [`CancelKey::token`].
    #[must_use]
    pub fn accepts_cancel_token(&self, token: Fr) -> bool {
        poseidon2_compress(&[token]) == self.cancel_hash
    }
}

impl CancelKey {
    /// Proves cancellation to a relay without revealing the key.
    #[must_use]
    pub fn token(&self) -> Fr {
        poseidon2_compress(&[self.0])
    }

    /// The hash intents commit to.
    #[must_use]
    pub fn hash(&self) -> Fr {
        poseidon2_compress(&[self.token()])
    }
}

impl OperationBuilderIntentExt for OperationBuilder {
    fn fill(self, intent: &Intent) -> Self {
        self.spend(intent.note()).output_note(intent.want)
    }
}

impl std::fmt::Debug for CancelKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CancelKey([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Bytes;
    use ark_std::rand::{SeedableRng, rngs::StdRng};
    use tint::syncer::Committed;

    use super::*;

    #[test]
    fn new_is_valid() {
        let (intent, _) = intent();
        intent.validate().unwrap();
    }

    #[test]
    fn rejects_zero_amount() {
        let receiver = Account::from_seed(&[1; 32], Policy::NONE).pub_account();
        let asset = AssetId::default();
        let mut rng = StdRng::seed_from_u64(1);

        let err = Intent::new(
            Address::ZERO,
            (asset, 0),
            (asset, 1),
            &receiver,
            0,
            &mut rng,
        );
        assert!(matches!(err, Err(IntentError::ZeroAmount)));
    }

    #[test]
    fn rejects_changed_terms() {
        let (mut intent, _) = intent();
        intent.want.amount -= 1;

        assert!(matches!(intent.validate(), Err(IntentError::OfferMismatch)));
    }

    #[test]
    fn accepts_cancel_token() {
        let (intent, key) = intent();

        assert!(intent.accepts_cancel_token(key.token()));
        assert!(!intent.accepts_cancel_token(key.0));
    }

    #[test]
    fn serde_round_trip() {
        let (intent, _) = intent();

        let json = serde_json::to_string(&intent).unwrap();
        assert_eq!(serde_json::from_str::<Intent>(&json).unwrap(), intent);
    }

    #[test]
    fn received_once_committed() {
        let owner = Account::from_seed(&[1; 32], Policy::NONE);
        let (intent, _) = intent();
        let committed = |commitment| {
            let event = SyncEvent::Committed(Committed {
                commitment,
                leaf_index: 0,
                encrypted_note: Bytes::new(),
                block_number: 0,
            });
            Snapshot::new(Address::ZERO, 0..1, vec![event])
        };

        let received = intent.received(&committed(intent.want.hash()), &owner);
        assert_eq!(received.map(|n| n.note), Some(intent.want));
        assert!(
            intent
                .received(&committed(intent.offer.hash()), &owner)
                .is_none()
        );
    }

    fn intent() -> (Intent, CancelKey) {
        let receiver = Account::from_seed(&[1; 32], Policy::NONE).pub_account();
        let a = AssetId::from(Address::repeat_byte(1));
        let b = AssetId::from(Address::repeat_byte(2));
        let mut rng = StdRng::seed_from_u64(1);
        Intent::new(
            Address::repeat_byte(3),
            (a, 100),
            (b, 50),
            &receiver,
            1_000,
            &mut rng,
        )
        .unwrap()
    }
}

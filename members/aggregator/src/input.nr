use bb_proof_verification::{UltraHonkVerificationKey, UltraHonkZKProof};
use poseidon::poseidon2::Poseidon2;

use super::note::Note;

pub struct Input {
    pub note: Note,
    pub nullifier_privkey: Field,

    pub spend_vk: UltraHonkVerificationKey,
    pub spend_proof: UltraHonkZKProof,

    pub inclusion_vk: UltraHonkVerificationKey,
    pub inclusion_proof: UltraHonkZKProof,
    pub is_archive: bool,
}

impl Input {
    pub fn default() -> Self {
        Self {
            note: Note::default(),
            nullifier_privkey: 0,
            spend_vk: UltraHonkVerificationKey::default(),
            spend_proof: UltraHonkZKProof::default(),
            inclusion_vk: UltraHonkVerificationKey::default(),
            inclusion_proof: UltraHonkZKProof::default(),
            is_archive: false,
        }
    }

    pub fn commitment(self) -> Field {
        self.note.commitment()
    }

    pub fn nullifier(self) -> Field {
        self.verify_nullifier_key();
        Poseidon2::hash([self.nullifier_privkey, self.commitment()], 2)
    }

    fn verify_nullifier_key(self) {
        let computed_nullifier_pubkey = Poseidon2::hash([self.nullifier_privkey], 1);
        assert(computed_nullifier_pubkey == self.note.nullifier_pubkey, "Nullifier key mismatch");
    }
}

#[test(should_fail_with = "Nullifier key mismatch")]
fn test_nullifier_key_mismatch() {
    let mut input = Input::default();
    input.nullifier_privkey = 12345;
    input.note.nullifier_pubkey = 54321;

    let _ = input.nullifier();
}

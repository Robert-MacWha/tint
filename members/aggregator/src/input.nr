use super::note::Note;

pub struct Input {
    pub note: Note,
    pub nullifier_privkey: Field,

    pub spend_vk: [Field; 114],
    pub spend_proof: [Field; 93],

    pub inclusion_vk: [Field; 114],
    pub inclusion_proof: [Field; 93],
    pub is_archive: bool,
}

impl Input {
    pub fn default() -> Self {
        Self {
            note: Note::default(),
            nullifier_privkey: 0,
            spend_vk: [0; 114],
            spend_proof: [0; 93],
            inclusion_vk: [0; 114],
            inclusion_proof: [0; 93],
            is_archive: false,
        }
    }

    pub fn commitment(self) -> Field {
        self.note.commitment()
    }

    pub fn nullifier(self) -> Field {
        self.verify_nullifier_key();
        std::hash::poseidon::bn254::hash_2([self.nullifier_privkey, self.commitment()])
    }

    fn verify_nullifier_key(self) {
        let computed_nullifier_pubkey =
            std::hash::poseidon::bn254::hash_1([self.nullifier_privkey]);
        assert(computed_nullifier_pubkey == self.note.nullifier_pubkey, "Nullifier key mismatch");
    }
}

#[test(should_fail)]
fn test_nullifier_key_mismatch() {
    let mut input = Input::default();
    input.nullifier_privkey = 12345;
    input.note.nullifier_pubkey = 54321;

    let _ = input.nullifier(); // This should panic due to nullifier key mismatch
}

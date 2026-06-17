use poseidon::poseidon2::Poseidon2;

pub struct Note {
    pub asset_id: Field,
    pub amount: Field,
    pub spend_vk_hash: Field,
    pub salt: Field,

    //? When created notes are committed to a particular nullifier pubkey.
    //? The note can only be spent if the spender has the corresponding nullifier privkey,
    //? which is used to compute deterministic & private nullifiers for the note.
    pub nullifier_pubkey: Field,
}

impl Note {
    pub fn default() -> Self {
        Self { asset_id: 0, amount: 0, spend_vk_hash: 0, salt: 0, nullifier_pubkey: 0 }
    }

    pub fn commitment(self) -> Field {
        Poseidon2::hash(
            [self.asset_id, self.amount, self.spend_vk_hash, self.salt, self.nullifier_pubkey],
            5,
        )
    }
}

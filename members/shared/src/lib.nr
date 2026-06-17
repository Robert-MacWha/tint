pub struct Note {
    asset_id: Field,
    amount: Field,
    owner_vk_hash: Field,
    salt: Field,
}

pub fn compute_commitment(note: Note) -> Field {
    std::hash::poseidon2::Poseidon2::hash(
        [note.asset_id, note.amount, note.owner_vk_hash, note.salt],
        4,
    )
}


pub struct Note {
    asset_id: Field,
    amount: Field,
    owner_vk_hash: Field,
    salt: Field,
}

pub fn compute_commitment(note: Note) -> Field {
    std::hash::poseidon2_permutation(input)
}

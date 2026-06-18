fn main(nullifier: pub Field) {
    let hash = poseidon::poseidon2::Poseidon2::hash([nullifier], 1);
    assert(hash != 0);
}

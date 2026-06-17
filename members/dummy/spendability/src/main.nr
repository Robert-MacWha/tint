fn main(nullifier: pub Field) {
    let hash = std::hash::poseidon::bn254::hash_1([nullifier]);
    assert(hash != 0);
}

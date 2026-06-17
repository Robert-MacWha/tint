fn main(nullifier: pub Field) {
    let hash = std::hash::pedersen_hash([nullifier]);
    assert(hash != 0);
}

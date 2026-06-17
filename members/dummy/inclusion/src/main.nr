fn main(root: pub Field, leaf: pub Field) {
    let hash = std::hash::poseidon::bn254::hash_2([root, leaf]);
    assert(hash != 0);
}

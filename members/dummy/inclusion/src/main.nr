fn main(root: pub Field, leaf: pub Field) {
    let hash = std::hash::pedersen_hash([root, leaf]);
    assert(hash != 0);
}

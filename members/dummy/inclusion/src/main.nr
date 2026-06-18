fn main(root: pub Field, leaf: pub Field) {
    let hash = poseidon::poseidon2::Poseidon2::hash([root, leaf], 2);
    assert(hash != 0);
}

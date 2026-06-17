global DEPTH: u32 = 4;

fn main(root: pub Field, leaf: pub Field, siblings: [Field; DEPTH], indices: [u8; DEPTH]) {
    let mut current = leaf;

    for i in 0..DEPTH {
        let index = indices[i];
        let sibs = siblings[i];

        let preimage = if index == 0 {
            [current, sibs]
        } else {
            [sibs, current]
        };
        current = std::hash::poseidon2::Poseidon2::hash(preimage, 2);
    }

    assert(current == root, "Merkle proof does not match root");
}

use ::poseidon::poseidon2::Poseidon2;

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
        current = Poseidon2::hash(preimage, 2);
    }

    assert(current == root, "Merkle proof does not match root");
}

#[test]
fn test_inclusion_valid() {
    let leaf: Field = 1;
    let siblings: [Field; DEPTH] = [10, 20, 30, 40];
    let indices: [u8; DEPTH] = [0, 1, 0, 1];

    let h0 = Poseidon2::hash([leaf, siblings[0]], 2); // current left
    let h1 = Poseidon2::hash([siblings[1], h0], 2); // current right
    let h2 = Poseidon2::hash([h1, siblings[2]], 2); // current left
    let root = Poseidon2::hash([siblings[3], h2], 2); // current right

    main(root, leaf, siblings, indices);
}

#[test(should_fail_with = "Merkle proof does not match root")]
fn test_inclusion_wrong_root() {
    let leaf: Field = 1;
    let siblings: [Field; DEPTH] = [10, 20, 30, 40];
    let indices: [u8; DEPTH] = [0, 1, 0, 1];

    let h0 = Poseidon2::hash([leaf, siblings[0]], 2);
    let h1 = Poseidon2::hash([siblings[1], h0], 2);
    let h2 = Poseidon2::hash([h1, siblings[2]], 2);
    let root = Poseidon2::hash([siblings[3], h2], 2);

    main(root + 1, leaf, siblings, indices);
}

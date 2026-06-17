use ::keccak256::keccak256;

global DEPTH: u32 = 4;

fn main(
    root: pub [u8; 32],
    leaf: pub [u8; 32],
    siblings: [[u8; 32 * 3]; DEPTH],
    indices: [u8; DEPTH],
) {
    let mut current = leaf;

    for i in 0..DEPTH {
        let mut preimage = [0; 128];
        let index = indices[i];
        let sibs = siblings[i];
        for j in 0..32 {
            preimage[j] = if index == 0 { current[j] } else { sibs[j] };
            preimage[32 + j] = if index == 1 {
                current[j]
            } else if index == 0 {
                sibs[j]
            } else {
                sibs[32 + j]
            };
            preimage[64 + j] = if index == 2 {
                current[j]
            } else if index == 3 {
                sibs[64 + j]
            } else {
                sibs[32 + j]
            };
            preimage[96 + j] = if index == 3 { current[j] } else { sibs[64 + j] };
        }
        current = keccak256(preimage, preimage.len());
    }

    assert(current == root, "Merkle proof does not match root");
}

// Hash with `current` occupying the first 32-byte slot (matches index == 0 routing).
fn hash_current_first(current: [u8; 32], sibs: [u8; 96]) -> [u8; 32] {
    let mut preimage: [u8; 128] = [0; 128];
    for j in 0..32 {
        preimage[j] = current[j];
    }
    for j in 0..96 {
        preimage[32 + j] = sibs[j];
    }
    keccak256(preimage, preimage.len())
}

// Hash with `current` occupying the last 32-byte slot (matches index == 3 routing).
fn hash_current_last(current: [u8; 32], sibs: [u8; 96]) -> [u8; 32] {
    let mut preimage: [u8; 128] = [0; 128];
    for j in 0..96 {
        preimage[j] = sibs[j];
    }
    for j in 0..32 {
        preimage[96 + j] = current[j];
    }
    keccak256(preimage, preimage.len())
}

#[test]
fn test_inclusion_valid_current_first() {
    let leaf: [u8; 32] = [1; 32];
    let siblings: [[u8; 96]; DEPTH] = [[2; 96], [3; 96], [4; 96], [5; 96]];
    let indices: [u8; DEPTH] = [0, 0, 0, 0];

    let mut current = leaf;
    for i in 0..DEPTH {
        current = hash_current_first(current, siblings[i]);
    }

    main(current, leaf, siblings, indices);
}

#[test]
fn test_inclusion_valid_current_last() {
    let leaf: [u8; 32] = [7; 32];
    let siblings: [[u8; 96]; DEPTH] = [[2; 96], [3; 96], [4; 96], [5; 96]];
    let indices: [u8; DEPTH] = [3, 3, 3, 3];

    let mut current = leaf;
    for i in 0..DEPTH {
        current = hash_current_last(current, siblings[i]);
    }

    main(current, leaf, siblings, indices);
}

#[test(should_fail_with = "Merkle proof does not match root")]
fn test_inclusion_wrong_root() {
    let leaf: [u8; 32] = [1; 32];
    let siblings: [[u8; 96]; DEPTH] = [[2; 96], [3; 96], [4; 96], [5; 96]];
    let indices: [u8; DEPTH] = [0, 0, 0, 0];

    // A root that does not correspond to the witness must not verify.
    let wrong_root: [u8; 32] = [0; 32];
    main(wrong_root, leaf, siblings, indices);
}

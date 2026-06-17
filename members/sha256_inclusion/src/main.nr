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
        current = std::hash::sha256(preimage);
    }

    assert(current == root, "Merkle proof does not match root");
}

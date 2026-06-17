use dep::keccak256::keccak256;

// Adjust DEPTH here to benchmark different sizes
global DEPTH: u32 = 8;

// A 4-ary node needs 3 siblings (3 * 32 = 96 bytes) + the current hash (32 bytes)
fn main(
    leaf: [u8; 32],
    siblings: [[u8; 96]; DEPTH],
    indices: [u8; DEPTH], // 0, 1, 2, or 3
) -> pub [u8; 32] {
    let mut current = leaf;

    for i in 0..DEPTH {
        let mut preimage = [0; 128];
        let index = indices[i];
        let sibs = siblings[i];

        // Conditional routing: place `current` in the correct 32-byte slot based on index
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

    current
}

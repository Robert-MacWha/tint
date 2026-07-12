//! Regenerates `packages/contracts/src/Groth16Verifier.sol` from the dev
//! trusted setup's `VerifyingKey`. Run with `cargo run --bin gen_verifier -p tint-rs`
//! whenever the circuit shape changes.

use std::path::Path;

use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint_rs::{codegen, provider};

fn main() {
    let mut rng = StdRng::seed_from_u64(provider::DEV_SETUP_SEED);
    let (_pk, vk) = provider::setup(&mut rng).expect("dev trusted setup failed");

    let solidity = codegen::groth16_verifier_solidity(&vk);

    let out_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/src/Groth16Verifier.sol");
    std::fs::write(&out_path, solidity).expect("failed to write Groth16Verifier.sol");

    println!("wrote {}", out_path.display());
}

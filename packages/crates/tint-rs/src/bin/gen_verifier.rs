//! Regenerates `packages/contracts/src/Groth16Verifier.sol` from the dev
//! trusted setup's `VerifyingKey`. Run with `cargo run --bin gen_verifier -p tint-rs`
//! whenever the circuit shape changes.

use std::path::Path;

use tint_rs::{circuit::setup_circuits, codegen};

fn main() {
    let (_pk, vk) = setup_circuits().unwrap();

    println!("Generating Groth16Verifier.sol");
    let solidity = codegen::groth16_verifier_solidity(&vk);

    let out_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/src/Groth16Verifier.sol");
    std::fs::write(&out_path, solidity).expect("failed to write Groth16Verifier.sol");

    println!("wrote {}", out_path.display());
}

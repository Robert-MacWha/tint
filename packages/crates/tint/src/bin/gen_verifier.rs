//! Generates `packages/contracts/src/Groth16Verifier.sol` from the dev
//! trusted setup's `VerifyingKey`. Run with `cargo run --bin gen_verifier`
//! whenever the circuit shape changes.

use std::path::Path;

use tint::{
    circuit::{generate_artifacts, join_split::JoinSplit},
    codegen,
};

fn main() {
    let (_matrices, _pk, vk) = generate_artifacts::<JoinSplit>().unwrap();

    println!("Generating Groth16Verifier.sol");
    let solidity = codegen::groth16_verifier_solidity(&vk, "Groth16Verifier", true);

    let out_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/src/Groth16Verifier.sol");
    std::fs::write(&out_path, solidity).expect("failed to write Groth16Verifier.sol");

    println!("wrote {}", out_path.display());
}

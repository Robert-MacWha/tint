//! Regenerates `packages/contracts/src/spendability/PasswordSpendabilityVerifier.sol` from
//! the dev trusted setup's `VerifyingKey`. Run with `cargo run -p
//! tint-spendability --bin gen_password_verifier` whenever the circuit shape
//! changes.

use std::path::Path;

use tint::{circuit::setup_circuit, codegen};
use tint_spendability::circuit::password::PasswordSpendability;

fn main() {
    let (_pk, vk) = setup_circuit::<PasswordSpendability>().unwrap();

    println!("Generating PasswordSpendabilityVerifier.sol");
    let solidity = codegen::groth16_verifier_solidity(&vk, "PasswordSpendabilityVerifier", false);

    let out_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/src/spendability/PasswordSpendabilityVerifier.sol");
    std::fs::write(&out_path, solidity).expect("failed to write PasswordSpendabilityVerifier.sol");

    println!("wrote {}", out_path.display());
}

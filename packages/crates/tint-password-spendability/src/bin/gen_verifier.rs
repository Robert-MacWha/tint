//! Regenerates `packages/contracts/src/spendability/PasswordSpendabilityVerifier.sol` from
//! the dev trusted setup's `VerifyingKey`. Run with `cargo run -p
//! tint-spendability --bin gen_password_verifier` whenever the circuit shape
//! changes.

use std::path::Path;

use taceo_groth16_sol::{SolidityVerifierConfig, SolidityVerifierContext, askama::Template};
use tint_groth16::artifacts::Artifacts;
use tint_password_spendability::circuit::PasswordSpendability;

#[allow(clippy::expect_used)]
fn main() {
    let artifacts = Artifacts::generate_deterministic::<PasswordSpendability>()
        .expect("failed to generate artifacts");

    println!("Generating PasswordVerifier.sol");
    let config = SolidityVerifierConfig::default();
    let contract = SolidityVerifierContext {
        vk: artifacts.vk,
        config,
    };

    let rendered = contract
        .render()
        .expect("failed to render PasswordVerifier.sol");

    let out_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/src/codegen/PasswordVerifier.sol");
    std::fs::create_dir_all(out_path.parent().unwrap()).expect("failed to create output directory");
    std::fs::write(&out_path, rendered).expect("failed to write PasswordVerifier.sol");

    println!("wrote {}", out_path.display());
}

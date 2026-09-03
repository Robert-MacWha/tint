//! Generates `packages/contracts/src/Groth16Verifier.sol` from the dev
//! trusted setup's `VerifyingKey`.

use std::path::Path;

use taceo_groth16_sol::{SolidityVerifierConfig, SolidityVerifierContext, askama::Template};
use tint::circuit::join_split::JoinSplit;
use tint_groth16::artifacts::Artifacts;

#[allow(clippy::expect_used)]
fn main() {
    let artifacts = Artifacts::generate_deterministic::<JoinSplit>()
        .expect("failed to generate JoinSplit artifacts");

    println!("Generating TintVerifier.sol");
    let config = SolidityVerifierConfig::default();
    let contract = SolidityVerifierContext {
        vk: artifacts.vk,
        config,
    };
    let rendered = contract
        .render()
        .expect("failed to render TintVerifier.sol");

    let out_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/src/codegen/TintVerifier.sol");
    std::fs::create_dir_all(out_path.parent().unwrap()).expect("failed to create output directory");
    std::fs::write(&out_path, rendered).expect("failed to write TintVerifier.sol");

    println!("wrote {}", out_path.display());
}

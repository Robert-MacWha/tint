//! Regenerates `packages/contracts/src/codegen/TimelockVerifier.sol` from
//! the dev trusted setup's `VerifyingKey`. Run with `cargo run -p
//! tint-timelock-policy --features codegen --bin gen_verifier` whenever the
//! circuit shape changes.

use std::path::Path;

use taceo_groth16_sol::{SolidityVerifierConfig, SolidityVerifierContext, askama::Template};
use tint_groth16::artifacts::Artifacts;
use tint_timelock_policy::circuit::TimelockPolicyCircuit;

#[expect(clippy::expect_used, clippy::unwrap_used)]
fn main() {
    let artifacts = Artifacts::generate_deterministic::<TimelockPolicyCircuit>()
        .expect("failed to generate artifacts");

    println!("Generating TimelockVerifier.sol");
    let config = SolidityVerifierConfig::default();
    let contract = SolidityVerifierContext {
        vk: artifacts.vk,
        config,
    };

    let rendered = contract
        .render()
        .expect("failed to render TimelockVerifier.sol");

    let out_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/src/codegen/TimelockVerifier.sol");
    std::fs::create_dir_all(out_path.parent().unwrap()).expect("failed to create output directory");
    std::fs::write(&out_path, rendered).expect("failed to write TimelockVerifier.sol");

    println!("wrote {}", out_path.display());
}

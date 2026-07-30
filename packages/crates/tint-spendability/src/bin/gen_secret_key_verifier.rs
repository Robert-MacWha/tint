//! Regenerates `packages/contracts/src/spendability/SecretKeySpendabilityVerifier.sol` from
//! the dev trusted setup's `VerifyingKey`. Run with `cargo run -p
//! tint-spendability --bin gen_secret_key_verifier` whenever the circuit shape
//! changes.

use std::path::Path;

use tint::{circuit::setup_circuit, codegen};
use tint_spendability::circuit::secret_key::SecretKeySpendability;

fn main() {
    let (_pk, vk) = setup_circuit::<SecretKeySpendability>().unwrap();

    println!("Generating SecretKeySpendabilityVerifier.sol");
    let solidity = codegen::groth16_verifier_solidity(&vk, "SecretKeySpendabilityVerifier", false);

    let out_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/src/spendability/SecretKeySpendabilityVerifier.sol");
    std::fs::write(&out_path, solidity).expect("failed to write SecretKeySpendabilityVerifier.sol");

    println!("wrote {}", out_path.display());
}

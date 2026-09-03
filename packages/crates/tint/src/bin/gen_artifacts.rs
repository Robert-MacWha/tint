//! Generates groth16 proving & verifying artifacts for the `JoinSplit` circuit,
//! compresses them, and writes them to disk.

use tint::circuit::join_split::JoinSplit;
use tint_groth16::{
    artifacts::Artifacts,
    serde::{serialize_matrices, serialize_pk, serialize_vk},
};

const ARTIFACTS_DIR: &str = "artifacts/";

#[allow(clippy::expect_used)]
fn main() {
    println!("Generating artifacts");
    let artifacts = Artifacts::generate_deterministic::<JoinSplit>()
        .expect("failed to generate JoinSplit artifacts");

    println!("Serializing artifacts");
    let pk_bytes = serialize_pk(&artifacts.pk).expect("failed to serialize proving key");
    let vk_bytes = serialize_vk(&artifacts.vk).expect("failed to serialize verifying key");
    let matrices_bytes =
        serialize_matrices(&artifacts.matrices).expect("failed to serialize matrices");

    std::fs::create_dir_all(ARTIFACTS_DIR).expect("failed to create artifacts directory");
    write_to_file(
        format!("{ARTIFACTS_DIR}proving_key.bin.br").as_str(),
        &pk_bytes,
    );
    write_to_file(
        format!("{ARTIFACTS_DIR}verifying_key.bin.br").as_str(),
        &vk_bytes,
    );
    write_to_file(
        format!("{ARTIFACTS_DIR}matrices.bin.br").as_str(),
        &matrices_bytes,
    );

    println!("Done generating artifacts");
}

#[allow(clippy::expect_used)]
fn write_to_file(path: &str, data: &[u8]) {
    println!("Writing compressed data to {path}");
    std::fs::write(path, data).expect("failed to write compressed data to file");
}

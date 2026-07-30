//! Generates groth16 proving & verifying artifacts for the JoinSplit circuit,
//! compresses them, and writes them to disk.
//!
//! Run with `cargo run --release --bin gen_artifacts`.

use ark_bn254::Fr;
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, OptimizationGoal, SynthesisMode,
};
use tint::circuit::{artifacts, join_split::JoinSplit, matrices::Matrices, setup_circuit};

const ARTIFACTS_DIR: &str = "artifacts/";

fn main() {
    println!("Generating proving and verifying keys");
    let (pk, vk) = setup_circuit::<JoinSplit>().unwrap();

    println!("Generating constraint matrices");
    let circuit = JoinSplit::default();
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);
    cs.set_mode(SynthesisMode::Setup);
    circuit.clone().generate_constraints(cs.clone()).unwrap();
    cs.finalize();
    let matrices = Matrices::from_constraint_system(&cs).unwrap();

    println!("Serializing artifacts");
    let pk_bytes = artifacts::serialize_proving_key(&pk).unwrap();
    let vk_bytes = artifacts::serialize_verifying_key(&vk).unwrap();
    let matrices_bytes = artifacts::serialize_matrices(&matrices).unwrap();

    std::fs::create_dir_all(ARTIFACTS_DIR).unwrap();
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

fn write_to_file(path: &str, data: &[u8]) {
    println!("Writing compressed data to {}", path);
    std::fs::write(path, data).unwrap();
}

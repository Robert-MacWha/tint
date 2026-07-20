//! Generates groth16 proving & verifying artifacts for the JoinSplit circuit,
//! compresses them, and writes them to disk.
//!
//! Run with `cargo run --release --bin gen_artifacts`.

use std::io::Write;

use ark_bn254::Fr;
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, OptimizationGoal, SynthesisMode,
};
use ark_serialize::CanonicalSerialize;
use brotli::CompressorWriter;
use tint::circuit::{join_split::JoinSplit, matrices::Matrices, setup_circuits};

const ARTIFACTS_DIR: &str = "artifacts/";

const BUFFER_SIZE: usize = 4096;
const Q: u32 = 11;
const LGWIN: u32 = 22;

fn main() {
    println!("Generating proving and verifying keys");
    let (pk, vk) = setup_circuits().unwrap();

    println!("Generating constraint matrices");
    let circuit = JoinSplit::default();
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);
    cs.set_mode(SynthesisMode::Setup);
    circuit.clone().generate_constraints(cs.clone()).unwrap();
    cs.finalize();
    let matrices = Matrices::from_constraint_system(&cs).unwrap();

    println!("Serializing artifacts");
    let mut pk_bytes = Vec::new();
    let mut vk_bytes = Vec::new();

    pk.serialize_uncompressed(&mut pk_bytes).unwrap();
    vk.serialize_uncompressed(&mut vk_bytes).unwrap();
    let matrices_bytes = postcard::to_stdvec(&matrices).unwrap();

    std::fs::create_dir_all(ARTIFACTS_DIR).unwrap();
    write_compressed_to_file(
        format!("{ARTIFACTS_DIR}proving_key.bin.br").as_str(),
        &pk_bytes,
    );
    write_compressed_to_file(
        format!("{ARTIFACTS_DIR}verifying_key.bin.br").as_str(),
        &vk_bytes,
    );
    write_compressed_to_file(
        format!("{ARTIFACTS_DIR}matrices.bin.br").as_str(),
        &matrices_bytes,
    );

    println!("Done generating artifacts");
}

fn write_compressed_to_file(path: &str, data: &[u8]) {
    println!("Writing compressed data to {}", path);

    let mut compressed = Vec::new();
    CompressorWriter::new(&mut compressed, BUFFER_SIZE, Q, LGWIN)
        .write_all(data)
        .unwrap();
    std::fs::write(path, compressed).unwrap();
}

//! Benchmarks for Groth16 proving with matrices.
//!
//! Compares the performance of Groth16 proving using the arkworks baseline
//! implementation against pre-computed matrices for the JoinSplit circuit.

use std::io::Read;
use std::time::Instant;

use ark_bn254::Bn254;
use ark_groth16::{Groth16, ProvingKey};
use ark_serialize::CanonicalDeserialize;
use ark_snark::SNARK;
use ark_std::rand::{rngs::StdRng, SeedableRng};
use tint::circuit::join_split::JoinSplit;
use tint::circuit::matrices::{prove_with_matrices, Matrices};

const PK_PATH: &str = "artifacts/proving_key.bin.br";
const MATRICES_PATH: &str = "artifacts/matrices.bin.br";
const BUFFER_SIZE: usize = 4096;

fn main() {
    let mut rng = StdRng::seed_from_u64(42);

    let pk_compressed = std::fs::read(PK_PATH).unwrap();
    let matrices_compressed = std::fs::read(MATRICES_PATH).unwrap();

    assert!(pk_compressed.len() > 0, "Proving key file is empty");
    assert!(matrices_compressed.len() > 0, "Matrices file is empty");

    let mut pk_bytes = Vec::new();
    brotli::Decompressor::new(&pk_compressed[..], BUFFER_SIZE)
        .read_to_end(&mut pk_bytes)
        .unwrap();

    let mut matrices_bytes = Vec::new();
    brotli::Decompressor::new(&matrices_compressed[..], BUFFER_SIZE)
        .read_to_end(&mut matrices_bytes)
        .unwrap();

    let pk = ProvingKey::<Bn254>::deserialize_uncompressed_unchecked(&pk_bytes[..]).unwrap();
    let matrices: Matrices = postcard::from_bytes(&matrices_bytes).unwrap();

    let circuit = JoinSplit::default();

    let baseline_prove_start = Instant::now();
    let _baseline_proof = Groth16::<Bn254>::prove(&pk, circuit.clone(), &mut rng).unwrap();
    let baseline_prove_time = baseline_prove_start.elapsed();

    let matrices_prove_start = Instant::now();
    let _matrices_proof = prove_with_matrices(&pk, circuit.clone(), &matrices, &mut rng).unwrap();
    let matrices_prove_time = matrices_prove_start.elapsed();

    println!("groth16 prove (baseline): {:?}", baseline_prove_time);
    println!("groth16 prove (matrices): {:?}", matrices_prove_time);
}

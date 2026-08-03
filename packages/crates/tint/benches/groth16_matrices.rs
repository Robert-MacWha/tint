//! Benchmarks for Groth16 proving with matrices.
//!
//! Compares the performance of Groth16 proving using the arkworks baseline
//! implementation against pre-computed matrices for the JoinSplit circuit.

use std::time::Instant;

use ark_bn254::Bn254;
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::circuit::{generate_artifacts, join_split::JoinSplit, matrices::prove_with_matrices};

fn main() {
    let mut rng = StdRng::seed_from_u64(42);

    let (matrices, pk, _vk) = generate_artifacts::<JoinSplit>().unwrap();

    let circuit = JoinSplit::default();

    let baseline_prove_start = Instant::now();
    let _ = Groth16::<Bn254>::prove(&pk, circuit.clone(), &mut rng).unwrap();
    let baseline_prove_time = baseline_prove_start.elapsed();

    let matrices_prove_start = Instant::now();
    let _ = prove_with_matrices(&matrices, &pk, circuit.clone(), &mut rng).unwrap();
    let matrices_prove_time = matrices_prove_start.elapsed();

    println!("groth16 prove (baseline): {:?}", baseline_prove_time);
    println!("groth16 prove (matrices): {:?}", matrices_prove_time);
}

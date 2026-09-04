//! Benchmarks for Groth16 proving with matrices.
//!
//! Compares the performance of Groth16 proving using the arkworks baseline
//! implementation against pre-computed matrices for the JoinSplit circuit.

use std::time::Instant;

use ark_bn254::Bn254;
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::circuit::join_split::JoinSplitCircuit;
use tint_groth16::{artifacts::Artifacts, prove::prove_with_matrices};

fn main() {
    let mut rng = StdRng::seed_from_u64(42);

    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>().unwrap();

    let circuit = JoinSplitCircuit::default();

    let baseline_prove_start = Instant::now();
    let _ = Groth16::<Bn254>::prove(&artifacts.pk, circuit.clone(), &mut rng).unwrap();
    let baseline_prove_time = baseline_prove_start.elapsed();

    let matrices_prove_start = Instant::now();
    let _ = prove_with_matrices(&artifacts.matrices, &artifacts.pk, &circuit, &mut rng).unwrap();
    let matrices_prove_time = matrices_prove_start.elapsed();

    println!("groth16 prove (baseline): {:?}", baseline_prove_time);
    println!("groth16 prove (matrices): {:?}", matrices_prove_time);
}

use std::time::Instant;

use ark_bn254::Bn254;
use ark_groth16::Groth16;
use ark_relations::gr1cs::ConstraintSynthesizer;
use ark_snark::SNARK;
use ark_std::rand::{rngs::StdRng, SeedableRng};
use circuit_profiler::format::{self, OutputFormat};
use circuit_profiler::profiling::{profile_constraints, profile_time};
use tint::circuit::join_split::JoinSplit;

fn main() {
    let format = output_format_from_args(std::env::args());

    let mut rng = StdRng::seed_from_u64(42);
    let circuit = JoinSplit::default();

    let (pk, _vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), &mut rng).unwrap();
    let cs_profile = profile_constraints(|cs| circuit.clone().generate_constraints(cs));
    let time_profile = profile_time(|cs| circuit.clone().generate_constraints(cs));

    println!("circuit stats:");
    println!(
        "{:<20} {:>6}",
        "constraints:", cs_profile.stats.num_constraints
    );
    println!(
        "{:<20} {:>6}",
        "public inputs:",
        cs_profile.stats.num_instance_variables - 1
    );
    println!(
        "{:<20} {:>6}",
        "witness variables:", cs_profile.stats.num_witness_variables
    );
    println!("");

    println!("constraint breakdown:");
    format::render(cs_profile.nodes, format);
    println!();

    println!("synthesis time breakdown:");
    format::render(time_profile.nodes, format);
    println!();

    let start = Instant::now();
    let _proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).unwrap();
    println!("groth16 prove: {:?}", start.elapsed());
}

fn output_format_from_args(mut args: impl Iterator<Item = String>) -> OutputFormat {
    match args.nth(1).as_deref() {
        Some("--tree") => OutputFormat::Tree,
        Some("--folded") => OutputFormat::Folded,
        _ => OutputFormat::Flat,
    }
}

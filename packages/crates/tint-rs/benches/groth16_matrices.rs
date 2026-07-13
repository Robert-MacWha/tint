use std::time::Instant;

use ark_bn254::{Bn254, Fr};
use ark_ff::UniformRand;
use ark_groth16::Groth16;
use ark_groth16::r1cs_to_qap::LibsnarkReduction;
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, OptimizationGoal, R1CS_PREDICATE_LABEL,
    SynthesisMode,
};
use ark_snark::SNARK;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint_rs::circuit::join_split::JoinSplit;

fn main() {
    let mut rng = StdRng::seed_from_u64(42);

    // Mock witness: proving time and constraint counts depend only on the
    // circuit's structure (fixed by N_INPUTS/N_OUTPUTS/N_WITHDRAWALS), not on
    // the specific values being proven, so default (zero-valued) values give
    // a representative benchmark without needing a real deposit/spend.
    let circuit = JoinSplit::default();

    // Trusted setup: computed once, not part of the timed benchmark.
    let (pk, _vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), &mut rng).unwrap();

    let artifact_gen_start = Instant::now();
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);
    cs.set_mode(SynthesisMode::Prove {
        construct_matrices: true,
        generate_lc_assignments: false,
    });
    circuit.clone().generate_constraints(cs.clone()).unwrap();
    cs.finalize();
    let matrices = cs.to_matrices().unwrap()[R1CS_PREDICATE_LABEL].clone();
    let num_inputs = cs.num_instance_variables();
    let num_constraints = cs.num_constraints();
    let num_witness_variables = cs.num_witness_variables();
    let artifact_gen_time = artifact_gen_start.elapsed();

    println!("circuit stats:");
    println!("  constraints:       {}", num_constraints);
    println!("  public inputs:     {}", num_inputs - 1);
    println!("  witness variables: {}", num_witness_variables);
    println!();
    println!("artifact generation: {:?}", artifact_gen_time);
    println!();

    let matrices_prove_start = Instant::now();
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);
    cs.set_mode(SynthesisMode::Prove {
        construct_matrices: false,
        generate_lc_assignments: false,
    });
    circuit.clone().generate_constraints(cs.clone()).unwrap();

    // Cheap safety net against circuit nondeterminism: the matrices generated
    // above are only valid for this witness synthesis if the circuit's shape
    // didn't change between the two passes. `num_constraints()` can't be used
    // here -- with `construct_matrices: false` the predicate systems never
    // record constraints (see `ConstraintSystem::enforce_constraint_arity_3`
    // and friends), so it's always 0 in this mode.
    assert_eq!(cs.num_instance_variables(), num_inputs);
    assert_eq!(cs.num_witness_variables(), num_witness_variables);

    let prover = cs.borrow().unwrap();
    let full_assignment = [
        prover.instance_assignment().unwrap(),
        prover.witness_assignment().unwrap(),
    ]
    .concat();
    drop(prover);

    let r = Fr::rand(&mut rng);
    let s = Fr::rand(&mut rng);
    let _matrices_proof = Groth16::<Bn254, LibsnarkReduction>::create_proof_with_reduction_and_matrices(
        &pk,
        r,
        s,
        &matrices,
        num_inputs,
        num_constraints,
        &full_assignment,
    )
    .unwrap();
    let matrices_prove_time = matrices_prove_start.elapsed();
    println!("groth16 prove (matrices): {:?}", matrices_prove_time);

    let baseline_prove_start = Instant::now();
    let _baseline_proof = Groth16::<Bn254>::prove(&pk, circuit.clone(), &mut rng).unwrap();
    let baseline_prove_time = baseline_prove_start.elapsed();
    println!("groth16 prove (baseline): {:?}", baseline_prove_time);
}

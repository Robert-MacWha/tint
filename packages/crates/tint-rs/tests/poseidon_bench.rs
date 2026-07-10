// use std::hint::black_box;

// use ark_bn254::{Bn254, Fr};
// use ark_crypto_primitives::{
//     crh::{
//         CRHSchemeGadget,
//         poseidon::{CRH, constraints::CRHGadget},
//     },
//     sponge::poseidon::PoseidonConfig,
// };
// use ark_groth16::Groth16;
// use ark_r1cs_std::{alloc::AllocVar, fields::fp::FpVar};
// use ark_relations::gr1cs::{
//     ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, OptimizationGoal,
//     R1CS_PREDICATE_LABEL, SynthesisError, SynthesisMode,
// };
// use ark_snark::SNARK;
// use ark_std::{
//     UniformRand,
//     rand::{SeedableRng, rngs::StdRng},
// };
// use web_time::Instant;

// #[cfg(target_arch = "wasm32")]
// wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_node_experimental);

// type ConstraintF = Fr;

// macro_rules! log {
//     ($($t:tt)*) => {{
//         #[cfg(not(target_arch = "wasm32"))]
//         eprintln!($($t)*);
//         #[cfg(target_arch = "wasm32")]
//         wasm_bindgen_test::console_log!($($t)*);
//     }};
// }

// // Builds PoseidonConfig for the given rate (capacity=1, so state width t=rate+1).
// // MDS and ARK constants are random with a fixed seed — values don't affect constraint count.
// fn poseidon_params(rate: usize) -> PoseidonConfig<Fr> {
//     let t = rate + 1;
//     let full_rounds = 8;
//     let partial_rounds = match rate {
//         2 => 57,
//         4 => 60,
//         8 => 65,
//         16 => 72,
//         _ => panic!("unsupported rate: {rate}"),
//     };
//     let mut rng = StdRng::seed_from_u64(0);
//     let mds: Vec<Vec<Fr>> = (0..t)
//         .map(|_| (0..t).map(|_| Fr::rand(&mut rng)).collect())
//         .collect();
//     let ark_consts: Vec<Vec<Fr>> = (0..full_rounds + partial_rounds)
//         .map(|_| (0..t).map(|_| Fr::rand(&mut rng)).collect())
//         .collect();
//     PoseidonConfig::new(full_rounds, partial_rounds, 5, mds, ark_consts, rate, 1)
// }

// #[derive(Clone)]
// struct PoseidonBenchCircuit {
//     params: PoseidonConfig<Fr>,
//     inputs: Vec<Vec<Fr>>, // 512 groups, each of length `rate`
// }

// impl ConstraintSynthesizer<Fr> for PoseidonBenchCircuit {
//     fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
//         let params_var = <CRHGadget<ConstraintF> as CRHSchemeGadget<
//             CRH<ConstraintF>,
//             ConstraintF,
//         >>::ParametersVar::new_constant(cs.clone(), &self.params)?;

//         for group in &self.inputs {
//             let input_vars: Vec<FpVar<ConstraintF>> = group
//                 .iter()
//                 .map(|x| FpVar::new_witness(cs.clone(), || Ok(*x)))
//                 .collect::<Result<_, _>>()?;
//             let _ = <CRHGadget<ConstraintF> as CRHSchemeGadget<CRH<ConstraintF>, ConstraintF>>::evaluate(
//                 &params_var,
//                 &input_vars,
//             )?;
//         }

//         Ok(())
//     }
// }

// fn run_bench(rate: usize) {
//     let params = poseidon_params(rate);
//     let mut rng = StdRng::seed_from_u64(42);

//     const TOTAL_INPUTS: usize = 1024;
//     let n_hashes = TOTAL_INPUTS / rate;

//     let inputs: Vec<Vec<Fr>> = (0..n_hashes)
//         .map(|_| (0..rate).map(|_| Fr::rand(&mut rng)).collect())
//         .collect();

//     let circuit = PoseidonBenchCircuit { params, inputs };

//     // Print circuit sizes (Setup mode builds matrices without witness assignments).
//     {
//         let cs = ConstraintSystem::new_ref();
//         cs.set_mode(SynthesisMode::Setup);
//         circuit.clone().generate_constraints(cs.clone()).unwrap();
//         log!(
//             "--- {n_hashes} × {rate}-element Poseidon ({TOTAL_INPUTS} total inputs)  [constraints: {}, witnesses: {}, pub inputs: {}] ---",
//             cs.num_constraints(),
//             cs.num_witness_variables(),
//             cs.num_instance_variables(),
//         );
//     }

//     // Setup — not timed.
//     let (pk, _vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), &mut rng).unwrap();

//     // Witness generation only: allocates variable assignments without building constraint matrices.
//     let t0 = Instant::now();
//     {
//         let cs = ConstraintSystem::new_ref();
//         cs.set_optimization_goal(OptimizationGoal::Constraints);
//         cs.set_mode(SynthesisMode::Prove {
//             construct_matrices: false,
//             generate_lc_assignments: false,
//         });
//         black_box(circuit.clone()).generate_constraints(cs).unwrap();
//     }
//     let witness_time = t0.elapsed();

//     // Build the proving constraint system (mirrors ark-groth16's internal prover) so we can
//     // time the one-time `finalize()` (LC inlining — a setup cost) separately from the crypto.
//     let cs = ConstraintSystem::new_ref();
//     cs.set_optimization_goal(OptimizationGoal::Constraints);
//     cs.set_mode(SynthesisMode::Prove {
//         construct_matrices: true,
//         generate_lc_assignments: false,
//     });
//     circuit.generate_constraints(cs.clone()).unwrap();

//     // Inline LCs (setup only): grows with hash arity via dense MDS layers; excluded from prove.
//     let t0 = Instant::now();
//     cs.finalize();
//     let inline_time = t0.elapsed();

//     // Extract R1CS matrices and the full assignment [1, public.., witness..] — not timed.
//     let matrices = cs.to_matrices().unwrap().remove(R1CS_PREDICATE_LABEL).unwrap();
//     let num_inputs = cs.num_instance_variables();
//     let num_constraints = cs.num_constraints();
//     let full_assignment = [
//         cs.instance_assignment().unwrap(),
//         cs.witness_assignment().unwrap(),
//     ]
//     .concat();

//     // Pure proving cryptography: QAP witness map + MSMs, no constraint synthesis or finalize.
//     let r = Fr::rand(&mut rng);
//     let s = Fr::rand(&mut rng);
//     let t0 = Instant::now();
//     let _ = black_box(
//         Groth16::<Bn254>::create_proof_with_reduction_and_matrices(
//             &pk,
//             r,
//             s,
//             &matrices,
//             num_inputs,
//             num_constraints,
//             &full_assignment,
//         )
//         .unwrap(),
//     );
//     let prove_time = t0.elapsed();

//     log!("  witness gen:        {witness_time:?}");
//     log!("  inline LCs (setup): {inline_time:?}");
//     log!("  prove (crypto):     {prove_time:?}");
// }

// #[cfg_attr(not(target_arch = "wasm32"), test)]
// #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
// fn bench_2_elements() {
//     run_bench(2);
// }

// #[cfg_attr(not(target_arch = "wasm32"), test)]
// #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
// fn bench_4_elements() {
//     run_bench(4);
// }

// #[cfg_attr(not(target_arch = "wasm32"), test)]
// #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
// fn bench_8_elements() {
//     run_bench(8);
// }

// // #[cfg_attr(not(target_arch = "wasm32"), test)]
// // #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
// // fn bench_16_elements() {
// //     run_bench(16);
// // }

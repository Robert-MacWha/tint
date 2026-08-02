// //! Dump of `taceo-poseidon2`'s BN254 round constants (T=2,3,8) as Go
// //! source, so they can be ported into
// //! `tint-multisig-spendability/go/internal/poseidon2/constants.go`
// //!
// //! cargo test --test dump_poseidon2_constants -- --ignored --nocapture

// use ark_bn254::Fr;
// use taceo_poseidon2::bn254::{t2, t3};
// use tint::circuit::poseidon2::poseidon2_compress;

// fn dump<const T: usize, const ROUNDS_P: usize>(
//     name: &str,
//     external: &[[Fr; T]; 8],
//     internal: &[Fr; ROUNDS_P],
//     diag: &[Fr; T],
// ) {
//     println!("var {name}External = [8][{T}]string{{");
//     for row in external {
//         print!("\t{{");
//         for c in row {
//             print!("{:?}, ", c.to_string());
//         }
//         println!("}},");
//     }
//     println!("}}");

//     print!("var {name}Internal = [{ROUNDS_P}]string{{");
//     for c in internal {
//         print!("{:?}, ", c.to_string());
//     }
//     println!("}}");

//     print!("var {name}Diag = [{T}]string{{");
//     for c in diag {
//         print!("{:?}, ", c.to_string());
//     }
//     println!("}}");
//     println!();
// }

// #[test]
// #[ignore]
// fn dump_poseidon2_constants() {
//     dump::<2, 56>(
//         "t2",
//         &t2::POSEIDON2_BN254_T2_PARAMS.round_constants_external,
//         &t2::POSEIDON2_BN254_T2_PARAMS.round_constants_internal,
//         &t2::POSEIDON2_BN254_T2_PARAMS.mat_internal_diag_m_1,
//     );
//     dump::<3, 56>(
//         "t3",
//         &t3::POSEIDON2_BN254_T3_PARAMS.round_constants_external,
//         &t3::POSEIDON2_BN254_T3_PARAMS.round_constants_internal,
//         &t3::POSEIDON2_BN254_T3_PARAMS.mat_internal_diag_m_1,
//     );

//     // Independent test vectors for cross-checking a from-scratch Go port,
//     // separate from the constants themselves.
//     println!(
//         "// compress2([1,2]) = {}",
//         poseidon2_compress(&[Fr::from(1u64), Fr::from(2u64)])
//     );
//     println!(
//         "// compress3([1,2,3]) = {}",
//         poseidon2_compress(&[Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)])
//     );
// }

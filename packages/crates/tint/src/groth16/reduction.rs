use ark_ec::pairing::Pairing;
use ark_ff::{FftField, Field};
use ark_poly::{EvaluationDomain, GeneralEvaluationDomain};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use tracing::instrument;

use crate::circuit::matrices::Matrices;

/// This trait is used to convert the witness into QAP witness as part of a Groth16 proof.
/// Refer to <https://docs.rs/ark-groth16/latest/ark_groth16/r1cs_to_qap/trait.R1CSToQAP.html>
/// for more details.  We do not implement the other methods of the arkworks trait,
/// as we do not need them during proof generation.
pub trait R1CSToQAP {
    /// Computes a QAP witness corresponding to the R1CS witness, using the
    /// provided `ConstraintMatrices`.
    fn witness_map_from_matrices<P: Pairing>(
        matrices: &Matrices<P::ScalarField>,
        witness: &[P::ScalarField],
    ) -> anyhow::Result<Vec<P::ScalarField>>;
}

fn evaluate_constraint<P: Pairing>(
    domain_size: usize,
    matrix: &Vec<Vec<(P::ScalarField, usize)>>,
    num_constraints: usize,
    witness: &[P::ScalarField],
) -> Vec<P::ScalarField> {
    let mut result = matrix
        .par_iter()
        .take(num_constraints)
        .map(|lhs| {
            let mut acc = P::ScalarField::default();
            for (coeff, index) in lhs {
                acc += *coeff * witness[*index];
            }
            acc
        })
        .collect::<Vec<_>>();
    result.resize(domain_size, P::ScalarField::default());
    result
}

/// Implements the witness map used by libsnark. The arkworks witness map
/// calculates the coefficients of H through computing (AB-C)/Z in the evaluation
/// domain and going back to the coefficients domain.
///
/// Based on <https://github.com/arkworks-rs/groth16/>.
pub struct LibSnarkReduction;

impl R1CSToQAP for LibSnarkReduction {
    #[instrument(level = "debug", name = "witness map from matrices", skip_all)]
    fn witness_map_from_matrices<P: Pairing>(
        matrices: &Matrices<P::ScalarField>,
        witness: &[P::ScalarField],
    ) -> anyhow::Result<Vec<P::ScalarField>> {
        let num_constraints = matrices.num_constraints;
        let num_inputs = matrices.num_inputs;
        let domain = GeneralEvaluationDomain::<P::ScalarField>::new(num_constraints + num_inputs)
            .ok_or(anyhow::anyhow!("Polynomial Degree too large"))?;
        let domain_size = domain.size();

        let coset_domain = domain
            .get_coset(P::ScalarField::GENERATOR)
            .expect("generator has always inverse");

        let (mut ab, c) = rayon::join(
            || {
                let (a, b) = rayon::join(
                    || {
                        let mut a = evaluate_constraint::<P>(
                            domain_size,
                            &matrices.matrices[0],
                            matrices.num_constraints,
                            witness,
                        );
                        a[num_constraints..num_constraints + num_inputs]
                            .clone_from_slice(&witness[..num_inputs]);
                        domain.ifft_in_place(&mut a);
                        coset_domain.fft_in_place(&mut a);
                        a
                    },
                    || {
                        let mut b = evaluate_constraint::<P>(
                            domain_size,
                            &matrices.matrices[1],
                            matrices.num_constraints,
                            witness,
                        );
                        domain.ifft_in_place(&mut b);
                        coset_domain.fft_in_place(&mut b);
                        b
                    },
                );
                a.iter()
                    .zip(b.iter())
                    .map(|(a, b)| *a * b)
                    .collect::<Vec<_>>()
            },
            || {
                let mut c = evaluate_constraint::<P>(
                    domain_size,
                    &matrices.matrices[2],
                    matrices.num_constraints,
                    witness,
                );
                domain.ifft_in_place(&mut c);
                coset_domain.fft_in_place(&mut c);
                c
            },
        );

        let vanishing_polynomial_over_coset = domain
            .evaluate_vanishing_polynomial(P::ScalarField::GENERATOR)
            .inverse()
            .expect("Inverse exists");

        ab.par_iter_mut().zip(c.par_iter()).for_each(|(ab_i, c_i)| {
            *ab_i -= *c_i;
            *ab_i *= vanishing_polynomial_over_coset;
        });

        coset_domain.ifft_in_place(&mut ab);

        Ok(ab)
    }
}

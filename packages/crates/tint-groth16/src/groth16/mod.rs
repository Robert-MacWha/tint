//! Adapted from <https://github.com/TaceoLabs/circom-helpers/tree/main/groth16>
//!
//! Original source code licensed under MIT.
//!
//! Updated to use arkworks 0.6.0 and remove circom compatibility code.
//!
//! TODO: Replace with importing the dep directly since they updated to 0.6.0

use std::marker::PhantomData;

use ark_ec::{AffineRepr, CurveGroup, VariableBaseMSM, pairing::Pairing};
pub use ark_groth16::{Proof, ProvingKey};
use ark_relations::gr1cs::SynthesisError;
use tracing::instrument;

use crate::matrices::Matrices;

mod reduction;
pub use reduction::LibSnarkReduction;

macro_rules! rayon_join5 {
    ($t1: expr, $t2: expr, $t3: expr, $t4: expr, $t5: expr) => {{
        let ((((v, w), x), y), z) = rayon::join(
            || rayon::join(|| rayon::join(|| rayon::join($t1, $t2), $t3), $t4),
            $t5,
        );
        (v, w, x, y, z)
    }};
}

/// A Groth16 proof protocol.
///
/// This struct should never be initialized, it only provides associated functions [`Groth16::prove`] and [`Groth16::verify`].
pub struct Groth16<P: Pairing> {
    phantom_data: PhantomData<P>,
}

impl<P: Pairing> Groth16<P> {
    #[instrument(level = "debug", name = "Groth16 - Proof", skip_all)]
    pub fn prove<R: reduction::R1CSToQAP>(
        pkey: &ProvingKey<P>,
        r: P::ScalarField,
        s: P::ScalarField,
        matrices: &Matrices<P::ScalarField>,
        witness: &[P::ScalarField],
    ) -> Result<Proof<P>, SynthesisError> {
        let witness_len = witness.len();
        let witness_should_len = matrices.num_witness_variables + matrices.num_inputs;
        if witness_len != witness_should_len {
            return Err(SynthesisError::AssignmentMissing);
        }
        let h = R::witness_map_from_matrices::<P>(matrices, witness)?;
        let proof =
            Self::create_proof_with_assignment(pkey, r, s, &h, witness, matrices.num_inputs)?;
        Ok(proof)
    }

    fn calculate_coeff<C>(
        initial: C,
        query: &[C::Affine],
        vk_param: C::Affine,
        witness: &[P::ScalarField],
    ) -> C
    where
        C: CurveGroup<ScalarField = P::ScalarField>,
    {
        let acc = C::msm_unchecked(&query[1..], witness);
        let mut res = initial;
        res += query[0].into_group();
        res += vk_param.into_group();
        res += acc;
        res
    }

    #[instrument(level = "debug", name = "create proof with assignment", skip_all)]
    fn create_proof_with_assignment(
        pkey: &ProvingKey<P>,
        r: P::ScalarField,
        s: P::ScalarField,
        h: &[P::ScalarField],
        witness: &[P::ScalarField],
        num_inputs: usize,
    ) -> Result<Proof<P>, SynthesisError> {
        let delta_g1 = pkey.delta_g1.into_group();
        let alpha_g1 = pkey.vk.alpha_g1;
        let beta_g1 = pkey.beta_g1;
        let beta_g2 = pkey.vk.beta_g2;
        let delta_g2 = pkey.vk.delta_g2.into_group();

        let (r_g1, s_g1, s_g2, l_acc, h_acc) = rayon_join5!(
            || {
                let compute_a =
                    tracing::debug_span!("compute A in create proof with assignment").entered();
                // Compute A
                let r_g1 = delta_g1 * r;
                let r_g1 = Self::calculate_coeff(r_g1, &pkey.a_query, alpha_g1, &witness[1..]);
                compute_a.exit();
                r_g1
            },
            || {
                let compute_b =
                    tracing::debug_span!("compute B/G1 in create proof with assignment").entered();
                // Compute B in G1
                // In original implementation this is skipped if r==0, however r is shared in our case
                let s_g1 = delta_g1 * s;
                let s_g1 = Self::calculate_coeff(s_g1, &pkey.b_g1_query, beta_g1, &witness[1..]);
                compute_b.exit();
                s_g1
            },
            || {
                let compute_b =
                    tracing::debug_span!("compute B/G2 in create proof with assignment").entered();
                // Compute B in G2
                let s_g2 = delta_g2 * s;
                let s_g2 = Self::calculate_coeff(s_g2, &pkey.b_g2_query, beta_g2, &witness[1..]);
                compute_b.exit();
                s_g2
            },
            || {
                let msm_l_query = tracing::debug_span!("msm l_query").entered();
                let result = P::G1::msm_unchecked(&pkey.l_query, &witness[num_inputs..]);
                msm_l_query.exit();
                result
            },
            || {
                let msm_h_query = tracing::debug_span!("msm h_query").entered();
                //perform the msm for h
                let result = P::G1::msm_unchecked(&pkey.h_query, h);
                msm_h_query.exit();
                result
            }
        );

        let rs = r * s;
        let r_s_delta_g1 = delta_g1 * rs;

        let g_a: P::G1 = r_g1;
        let g1_b = s_g1;

        let r_g1_b = g1_b * r;

        let s_g_a = g_a * s;

        let mut g_c = s_g_a;
        g_c += r_g1_b;
        g_c -= r_s_delta_g1;
        g_c += l_acc;

        g_c += h_acc;

        let g2_b: P::G2 = s_g2;

        Ok(Proof {
            a: g_a.into_affine(),
            b: g2_b.into_affine(),
            c: g_c.into_affine(),
        })
    }
}

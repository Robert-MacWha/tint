use std::marker::PhantomData;

use ark_crypto_primitives::crh::{CRHScheme, constraints::CRHSchemeGadget};
use ark_ff::PrimeField;
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, fields::fp::FpVar, prelude::GR1CSVar};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, OptimizationGoal, SynthesisError,
};

/// A circuit whose public inputs can be compressed via hybrid compression.
///
/// `verify` should witness `self` and enforce its relations, returning a structured
/// `Output` that can be flattened for compression.
pub trait CompressibleCircuit<F: PrimeField, const N: usize> {
    type Output: Flatten<F, N>;

    fn verify(&self, cs: &ConstraintSystemRef<F>) -> Result<Self::Output, SynthesisError>;
}

/// A circuit's flattened public statement.
///
/// The returned list of values is folded into `(alpha, beta, gamma)`.
pub trait Flatten<F: PrimeField, const N: usize> {
    fn flatten(&self) -> Result<[FpVar<F>; N], SynthesisError>;
}

/// Compressed circuit output, including the inner circuit's output.
pub struct Compressed<F, O> {
    pub output: O,
    pub stmt: Vec<F>,
    pub alpha: F,
    pub beta: F,
    pub gamma: F,
}

/// Compressed circuit wrapping an inner compressible circuit, folding its
/// public statement via hybrid compression.
#[derive(Clone, Debug, Default)]
pub struct CompressedCircuit<F, C, CRH, CRHGadget, const N: usize>
where
    F: PrimeField,
    C: CompressibleCircuit<F, N>,
    CRH: CRHScheme<Input = [F], Output = F>,
    CRHGadget: CRHSchemeGadget<CRH, F, InputVar = [FpVar<F>], OutputVar = FpVar<F>>,
{
    pub alpha: F,
    pub crh_params: CRH::Parameters,
    pub inner: C,
    _crh_gadget: PhantomData<CRHGadget>,
}

#[derive(Debug, thiserror::Error)]
pub enum CompressError {
    #[error(transparent)]
    Synthesis(#[from] SynthesisError),
    /// Stringified `ark_crypto_primitives::Error`: that type isn't `Sync`,
    /// so it's laundered into a `String` at the point it's produced.
    #[error("hybrid compression: {0}")]
    Compression(String),
}

impl<F, C, CRH, CRHGadget, const N: usize> CompressedCircuit<F, C, CRH, CRHGadget, N>
where
    F: PrimeField,
    C: CompressibleCircuit<F, N>,
    CRH: CRHScheme<Input = [F], Output = F>,
    CRHGadget: CRHSchemeGadget<CRH, F, InputVar = [FpVar<F>], OutputVar = FpVar<F>>,
{
    pub fn new(crh_params: CRH::Parameters, inner: C) -> Self {
        Self {
            alpha: F::default(),
            crh_params,
            inner,
            _crh_gadget: PhantomData,
        }
    }

    /// Runs inner's `verify` method to witness the circuit, then flattens and
    /// compresses its output.
    pub fn compress(&mut self) -> Result<Compressed<F, C::Output>, CompressError> {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);
        let output = self.inner.verify(&cs)?;
        cs.finalize();

        let stmt: Vec<F> = output
            .flatten()?
            .iter()
            .map(GR1CSVar::value)
            .collect::<Result<_, _>>()?;
        let (alpha, beta, gamma) = compress::<F, CRH>(&self.crh_params, &stmt)
            .map_err(|e| CompressError::Compression(e.to_string()))?;

        self.alpha = alpha;
        Ok(Compressed {
            output,
            stmt,
            alpha,
            beta,
            gamma,
        })
    }
}

impl<F, C, CRH, CRHGadget, const N: usize> ConstraintSynthesizer<F>
    for CompressedCircuit<F, C, CRH, CRHGadget, N>
where
    F: PrimeField,
    C: CompressibleCircuit<F, N>,
    CRH: CRHScheme<Input = [F], Output = F>,
    CRHGadget: CRHSchemeGadget<CRH, F, InputVar = [FpVar<F>], OutputVar = FpVar<F>>,
{
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        //? Compute and flatten the output
        let output = self.inner.verify(&cs)?;
        let stmt = output.flatten()?;

        //? Enforce hybrid compression of the flattened output
        let alpha_var = FpVar::new_input(cs.clone(), || Ok(self.alpha))?;
        let params_var = CRHGadget::ParametersVar::new_constant(cs.clone(), &self.crh_params)?;

        let (beta_var, gamma_var) = ark_hybrid_compression::constraints::hybrid_compression::<
            CRH,
            F,
            CRHGadget,
        >(&params_var, alpha_var, &stmt)?;

        //? Expose the `beta` and `gamma` outputs as public outputs, enforcing equality with the
        //? computed values.
        let beta_pub = FpVar::new_input(cs.clone(), || beta_var.value())?;
        beta_pub.enforce_equal(&beta_var)?;

        let gamma_pub = FpVar::new_input(cs, || gamma_var.value())?;
        gamma_pub.enforce_equal(&gamma_var)?;

        Ok(())
    }
}

/// Off-circuit hybrid compression: folds `stmt` into `(alpha, beta,
/// gamma)`.
pub fn compress<F, CRH>(
    crh_params: &CRH::Parameters,
    stmt: &[F],
) -> Result<(F, F, F), ark_crypto_primitives::Error>
where
    F: PrimeField,
    CRH: CRHScheme<Input = [F], Output = F>,
{
    let alpha = ark_hybrid_compression::KeccakCRH::<F>::evaluate(&(), stmt)?;
    let (beta, gamma) =
        ark_hybrid_compression::hybrid_compression::<CRH, F>(crh_params, alpha, stmt)?;
    Ok((alpha, beta, gamma))
}

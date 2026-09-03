use ark_ec::pairing::Pairing;
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_relations::gr1cs::ConstraintSynthesizer;
use ark_snark::SNARK;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use tracing::{info, warn};

use crate::matrices::Matrices;

#[derive(Clone, Debug)]
pub struct Artifacts<E: Pairing> {
    pub matrices: Matrices<E::ScalarField>,
    pub pk: ProvingKey<E>,
    pub vk: VerifyingKey<E>,
}

impl<E: Pairing> Artifacts<E> {
    /// This circuit setup is deterministic using a fixed seed. It is not cryptographically
    /// secure and should only be used for testing and development.
    pub fn generate_deterministic<C: ConstraintSynthesizer<E::ScalarField> + Default>()
    -> Result<Self, ark_relations::gr1cs::SynthesisError> {
        let mut rng = StdRng::seed_from_u64(42);

        warn!("Circuit setup with fixed seed. Only use for testing and development.");
        let (pk, vk) = Groth16::<E>::circuit_specific_setup(C::default(), &mut rng)?;
        let matrices = Matrices::generate::<C>()?;

        info!("Circuit setup complete.");
        Ok(Self { matrices, pk, vk })
    }
}

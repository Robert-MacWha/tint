use std::hint::black_box;

use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::{
    crh::{
        CRHSchemeGadget,
        poseidon::{
            CRH, TwoToOneCRH,
            constraints::{CRHGadget, TwoToOneCRHGadget},
        },
    },
    merkle_tree::{
        Config, IdentityDigestConverter, MerkleTree, Path,
        constraints::{ConfigGadget, PathVar},
    },
    sponge::poseidon::PoseidonConfig,
};
use ark_groth16::Groth16;
use ark_r1cs_std::{alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, OptimizationGoal, SynthesisError,
    SynthesisMode,
};
use ark_snark::SNARK;
use ark_std::{
    UniformRand,
    rand::{SeedableRng, rngs::StdRng},
    test_rng,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

type ConstraintF = Fr;

struct PoseidonMerkleTreeParams;

impl Config for PoseidonMerkleTreeParams {
    type Leaf = [Fr];
    type LeafDigest = Fr;
    type LeafInnerDigestConverter = IdentityDigestConverter<Fr>;
    type InnerDigest = Fr;
    type LeafHash = CRH<Fr>;
    type TwoToOneHash = TwoToOneCRH<Fr>;
}

type PoseidonMerkleTree = MerkleTree<PoseidonMerkleTreeParams>;

struct PoseidonMerkleTreeParamsVar;

impl ConfigGadget<PoseidonMerkleTreeParams, ConstraintF> for PoseidonMerkleTreeParamsVar {
    type Leaf = [FpVar<ConstraintF>];
    type LeafDigest = FpVar<ConstraintF>;
    type LeafInnerConverter = IdentityDigestConverter<FpVar<ConstraintF>>;
    type InnerDigest = FpVar<ConstraintF>;
    type LeafHash = CRHGadget<ConstraintF>;
    type TwoToOneHash = TwoToOneCRHGadget<ConstraintF>;
}

// BN254 Poseidon: t=3 (rate=2, capacity=1), standard round counts.
// MDS and ARK are random with a fixed seed — values don't affect constraint count.
fn poseidon_params() -> PoseidonConfig<Fr> {
    let mut rng = test_rng();
    let t = 3usize;
    let full_rounds = 8;
    let partial_rounds = 57;
    let mds: Vec<Vec<Fr>> = (0..t)
        .map(|_| (0..t).map(|_| Fr::rand(&mut rng)).collect())
        .collect();
    let ark: Vec<Vec<Fr>> = (0..full_rounds + partial_rounds)
        .map(|_| (0..t).map(|_| Fr::rand(&mut rng)).collect())
        .collect();
    PoseidonConfig::new(full_rounds, partial_rounds, 5, mds, ark, 2, 1)
}

// Path<P> doesn't impl Clone, so we store its fields separately.
#[derive(Clone)]
struct MembershipCircuit {
    params: PoseidonConfig<Fr>,
    root: Fr,
    leaf: Vec<Fr>,
    proof_sibling: Fr,
    proof_auth_path: Vec<Fr>,
    proof_leaf_index: usize,
}

impl MembershipCircuit {
    fn new(
        params: PoseidonConfig<Fr>,
        root: Fr,
        leaf: Vec<Fr>,
        proof: &Path<PoseidonMerkleTreeParams>,
    ) -> Self {
        Self {
            params,
            root,
            leaf,
            proof_sibling: proof.leaf_sibling_hash,
            proof_auth_path: proof.auth_path.clone(),
            proof_leaf_index: proof.leaf_index,
        }
    }
}

impl ConstraintSynthesizer<Fr> for MembershipCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let params_var =
            <CRHGadget<ConstraintF> as CRHSchemeGadget<CRH<ConstraintF>, _>>::ParametersVar::new_constant(
                cs.clone(),
                &self.params,
            )?;

        let root_var = FpVar::new_input(cs.clone(), || Ok(self.root))?;

        let leaf_var: Vec<FpVar<ConstraintF>> = self
            .leaf
            .iter()
            .map(|x| FpVar::new_input(cs.clone(), || Ok(*x)))
            .collect::<Result<_, _>>()?;

        let proof = Path::<PoseidonMerkleTreeParams> {
            leaf_sibling_hash: self.proof_sibling,
            auth_path: self.proof_auth_path,
            leaf_index: self.proof_leaf_index,
        };

        let path_var = PathVar::<
            PoseidonMerkleTreeParams,
            ConstraintF,
            PoseidonMerkleTreeParamsVar,
        >::new_witness(cs, || Ok(&proof))?;

        let is_member =
            path_var.verify_membership(&params_var, &params_var, &root_var, &leaf_var)?;

        is_member.enforce_equal(&Boolean::constant(true))
    }
}

fn benchmark_poseidon(c: &mut Criterion) {
    let params = poseidon_params();
    let mut rng = StdRng::seed_from_u64(42);

    let leaf_counts = [16777216];
    let circuits: Vec<_> = leaf_counts
        .iter()
        .map(|&n| {
            let leaves: Vec<Vec<Fr>> = (0..n).map(|i| vec![Fr::from(i as u64)]).collect();
            let tree = PoseidonMerkleTree::new(&params, &params, &leaves).unwrap();
            let root = tree.root();
            let proof = tree.generate_proof(0).unwrap();
            assert!(
                proof
                    .verify(&params, &params, &root, leaves[0].as_slice())
                    .unwrap()
            );
            MembershipCircuit::new(params.clone(), root, leaves[0].clone(), &proof)
        })
        .collect();

    // Print constraint counts once before benchmarking. Uses Setup mode which builds
    // constraint matrices without computing witness assignments.
    // num_constraints() = R1CS multiplication gates = circom's "non-linear constraints".
    // num_instance_variables() = public inputs; num_witness_variables() = private witnesses.
    eprintln!(
        "\n{:>10}  {:>8}  {:>8}  {:>8}  {:>8}",
        "leaves", "depth", "r1cs", "pub", "priv"
    );
    for (&num_leaves, circuit) in leaf_counts.iter().zip(circuits.iter()) {
        let cs = ConstraintSystem::new_ref();
        cs.set_mode(SynthesisMode::Setup);
        circuit.clone().generate_constraints(cs.clone()).unwrap();
        eprintln!(
            "{:>10}  {:>8}  {:>8}  {:>8}  {:>8}",
            num_leaves,
            (num_leaves as f64).log2() as usize,
            cs.num_constraints(),
            cs.num_instance_variables(),
            cs.num_witness_variables(),
        );
    }
    eprintln!();

    // Witness generation only: runs generate_constraints with construct_matrices: false,
    // so only variable assignments are computed — no constraint matrix construction.
    // This is the direct equivalent of circom's witness generation step.
    let mut witness_group = c.benchmark_group("poseidon_witness_gen");
    for (&num_leaves, circuit) in leaf_counts.iter().zip(circuits.iter()) {
        witness_group.bench_with_input(
            BenchmarkId::from_parameter(num_leaves),
            &num_leaves,
            |b, _| {
                b.iter(|| {
                    let cs = ConstraintSystem::new_ref();
                    cs.set_optimization_goal(OptimizationGoal::Constraints);
                    cs.set_mode(SynthesisMode::Prove {
                        construct_matrices: false,
                        generate_lc_assignments: false,
                    });
                    circuit.clone().generate_constraints(cs.clone()).unwrap();
                    black_box(cs.num_constraints())
                })
            },
        );
    }
    witness_group.finish();

    // Full Groth16 proof: witness gen + constraint matrix construction + QAP/H-poly + MSMs.
    // Setup (constraint synthesis → proving key) happens once outside the timed loop.
    let mut proof_group = c.benchmark_group("poseidon_groth16_prove");
    for (&num_leaves, circuit) in leaf_counts.iter().zip(circuits.iter()) {
        let (pk, _vk) =
            Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), &mut rng).unwrap();

        proof_group.bench_with_input(
            BenchmarkId::from_parameter(num_leaves),
            &num_leaves,
            |b, _| {
                b.iter(|| {
                    black_box(Groth16::<Bn254>::prove(&pk, circuit.clone(), &mut rng).unwrap())
                })
            },
        );
    }
    proof_group.finish();
}

criterion_group!(benches, benchmark_poseidon);
criterion_main!(benches);

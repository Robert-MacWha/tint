use alloy_sol_macro::sol;
use ark_bn254::Bn254;
use ark_groth16::Proof;

sol!(
    #[sol(ignore_unlinked)]
    Tint,
    "../../contracts/out/Tint.sol/Tint.json"
);

impl From<Proof<Bn254>> for ProofLib::Proof {
    fn from(p: Proof<Bn254>) -> Self {
        ProofLib::Proof {
            pA: [p.a.x.into(), p.a.y.into()],
            pB: [
                [p.b.x.c1.into(), p.b.x.c0.into()],
                [p.b.y.c1.into(), p.b.y.c0.into()],
            ],
            pC: [p.c.x.into(), p.c.y.into()],
        }
    }
}

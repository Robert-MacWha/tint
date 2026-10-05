use ark_bn254::Fr;
use ark_ff::UniformRand;
use ark_relations::gr1cs::ConstraintSystem;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::circuit::{FrVar, poseidon2::poseidon2_compress_gadget, witness};

fn main() {
    let mut rng = StdRng::seed_from_u64(42);

    println!("poseidon2_compress constraints:");
    println!("{:<8} {:>12}", "width", "constraints");
    for (t, count) in [
        (1, constraints::<1>(&mut rng)),
        (2, constraints::<2>(&mut rng)),
        (3, constraints::<3>(&mut rng)),
        (8, constraints::<8>(&mut rng)),
    ] {
        println!("{t:<8} {count:>12}");
    }
}

/// Counts the constraints emitted by a single `poseidon2_compress_gadget` of width `T`,
/// excluding the cost of allocating its inputs.
fn constraints<const T: usize>(rng: &mut StdRng) -> usize {
    let cs = ConstraintSystem::<Fr>::new_ref();

    let input: [FrVar; T] = std::array::from_fn(|_| {
        let value = Fr::rand(rng);
        witness(cs.clone(), &value).unwrap()
    });
    let before = cs.num_constraints();

    let _hash = poseidon2_compress_gadget(&input).unwrap();
    cs.num_constraints() - before
}

use alloy_primitives::{Address, U256};
use ark_std::rand::{Rng, rngs::StdRng};
use rand_core::SeedableRng;
use tint::{
    account::Account,
    circuit::join_split::JoinSplitCircuit,
    deposit::Deposit,
    note::asset::AssetId,
    op::OperationBuilder,
    policy::Policy,
    skew_mmr::{SkewMmr, SkewMmrExt},
    syncer::{Syncer, rpc::RpcSyncer},
};
use tint_groth16::artifacts::Artifacts;

use crate::common::anvil::{self, send};

mod common;

/// Shields, then transfers to a second account while unshielding and taking
/// change, using only the builder API.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn operation() {
    let mut rng = StdRng::seed_from_u64(1);
    let instance = anvil::setup().await.unwrap();
    let provider = instance.provider;
    let tint = *instance.tint.address();
    let token = instance.token;
    let artifacts = Artifacts::generate_deterministic::<JoinSplitCircuit>().unwrap();

    let alice = Account::from_seed(&[11u8; 32], Policy::NONE);
    let bob = Account::from_seed(&[12u8; 32], Policy::NONE);
    let unshield_address = Address::new(rng.r#gen());
    let asset = AssetId::from(*token.address());

    // Shield
    let deposit = Deposit::new(tint, alice.pub_account(), asset, 1_000, &mut rng);
    send(&provider, deposit.approval()).await;
    send(&provider, deposit.request(&mut rng).unwrap()).await;

    // Sync
    let syncer = RpcSyncer::new(provider.clone());
    let snapshot = syncer.sync(tint, ..).await.unwrap();
    let mut mmr = SkewMmr::new();
    mmr.append_events(&snapshot.events).unwrap();

    let notes = snapshot.unspent(&alice);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].note, deposit.note());

    // Transfer 300 to bob, unshield 200, and take 500 change.
    let op = OperationBuilder::new(tint)
        .spend(notes[0].clone())
        .output(bob.pub_account(), asset, 300)
        .withdraw(unshield_address, asset, 200)
        .change_to(alice.pub_account())
        .build(&mut rng)
        .unwrap();
    let policy_inputs = vec![Default::default(); op.inputs().len()];
    let proven = op
        .authorize(policy_inputs)
        .unwrap()
        .prove_with_mmr(&mmr, &artifacts, &mut rng)
        .unwrap();
    send(&provider, proven.into()).await;

    // Re-sync over the full range; already-appended leaves are skipped.
    let snapshot = syncer.sync(tint, ..).await.unwrap();
    mmr.append_events(&snapshot.events).unwrap();
    assert_eq!(mmr.len(), 3);

    let spent = snapshot.spent(&alice);
    assert_eq!(spent.len(), 1);
    assert_eq!(spent[0].note, deposit.note());

    let alice_notes = snapshot.unspent(&alice);
    assert_eq!(alice_notes.len(), 1);
    assert_eq!(alice_notes[0].note.amount, 500);

    let bob_notes = snapshot.unspent(&bob);
    assert_eq!(bob_notes.len(), 1);
    assert_eq!(bob_notes[0].note.amount, 300);

    assert_eq!(
        token.balanceOf(unshield_address).call().await.unwrap(),
        U256::from(200)
    );
}

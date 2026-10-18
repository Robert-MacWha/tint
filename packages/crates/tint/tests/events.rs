use alloy_primitives::Address;
use ark_std::rand::{Rng, rngs::StdRng};
use rand_core::SeedableRng;
use tint::{
    account::Account,
    operation::OperationBuilder,
    policy::Policy,
    syncer::{Committed, Nullified, SyncEvent, Withdrawn},
};
use tracing::info;

use crate::common::{
    anvil::{Instance, enable_simulation, send},
    util::{assert_unspent, init_tracing},
};

mod common;

/// An operation using only some of its input, output, and withdrawal slots
/// emits nullifiers, then commitments, then withdrawals, each in slot order,
/// and nothing for the sentinel-padded slots.
#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn events() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let mut rng = StdRng::seed_from_u64(1);
    let instance = Instance::setup().await?;

    let provider = instance.provider.clone();
    let tint = *instance.tint.address();
    let asset = instance.asset();

    let account_1 = Account::from_seed(&[11u8; 32], Policy::NONE);
    let account_2 = Account::from_seed(&[22u8; 32], Policy::NONE);
    let unshield_address = Address::new(rng.r#gen());

    // Deposit two notes into Tint
    let amount = 1_000u128;
    for _ in 0..2 {
        instance
            .deposit(account_1.pub_account(), amount, &mut rng)
            .await?;
    }

    let (snapshot, mmr) = instance.sync().await?;
    assert_unspent(&snapshot, &account_1, &[(asset, amount), (asset, amount)]);
    let notes = snapshot.unspent(&account_1);

    // The events don't depend on the proof, so set the simulation flag and
    // submit a simulated operation rather than proving one.
    enable_simulation(&provider, tint).await?;

    // Spend 2 inputs into 2 outputs and 1 withdrawal, leaving the rest of
    // the slots as sentinels.
    info!("Operating");
    let simulated = OperationBuilder::new()
        .spend(notes[0].clone())
        .spend(notes[1].clone())
        .output(account_1.pub_account(), asset, 300)
        .output(account_2.pub_account(), asset, 500)
        .withdraw(unshield_address, asset, 2 * amount - 800)
        .build(&mut rng)?
        .simulate_with_mmr(&mmr)?;
    let receipt = send(&provider, simulated.transaction(tint)).await?;

    // Verify emitted events
    info!("Verifying events");
    let block_number = receipt.block_number.ok_or("receipt missing block number")?;
    let emitted: Vec<SyncEvent> = receipt
        .inner
        .logs()
        .iter()
        .filter_map(SyncEvent::try_from_log)
        .collect();

    let ciphertexts = &simulated.operation().context.ciphertexts;
    let nullified = simulated.nullifiers().iter().map(|&nullifier| {
        SyncEvent::Nullified(Nullified {
            nullifier,
            block_number,
        })
    });

    // The two deposits occupy leaves 0 and 1.
    let committed = simulated.outputs().iter().zip(ciphertexts).zip(2u64..).map(
        |((output, ciphertext), leaf_index)| {
            SyncEvent::Committed(Committed {
                commitment: output.hash(),
                leaf_index,
                encrypted_note: ciphertext.clone(),
                block_number,
            })
        },
    );
    let withdrawn = SyncEvent::Withdrawn(Withdrawn {
        asset,
        amount: 2 * amount - 800,
        recipient: unshield_address,
        block_number,
    });
    let expected: Vec<SyncEvent> = nullified
        .chain(committed)
        .chain(std::iter::once(withdrawn))
        .collect();

    assert_eq!(simulated.nullifiers().len(), 2);
    assert_eq!(simulated.outputs().len(), 2);
    assert_eq!(emitted, expected);

    Ok(())
}

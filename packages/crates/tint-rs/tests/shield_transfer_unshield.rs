//! End-to-end shield/transfer/unshield test against a self-contained `anvil`
//! instance: this test spawns its own anvil node and deploys `Tint` +
//! `Groth16Verifier` + `MockToken` via `forge script`, so it runs standalone --
//! `cargo test --release --test shield_transfer_unshield` is all that's
//! needed (requires `forge` on `$PATH`).

mod common;

use alloy_node_bindings::Anvil;
use alloy_primitives::{Address, U256, hex};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use common::{IERC20, balance_of, deploy, make_indexer, send};
use tint_rs::{
    note::{asset::AssetId, keys::Keys, receiver::Receiver},
    provider::{self, Provider},
};

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn shield_transfer_unshield() {
    let anvil = Anvil::new().spawn();
    let rpc_url = anvil.endpoint();
    let deployer_key = hex::encode(anvil.first_key().to_bytes());
    let deployer_signer: PrivateKeySigner = format!("0x{deployer_key}").parse().unwrap();
    let recipient_address = anvil.addresses()[1];

    let (tint_address, token_address) = deploy(&rpc_url, &deployer_key);

    let eth_provider = ProviderBuilder::new()
        .wallet(deployer_signer)
        .connect_http(rpc_url.parse().unwrap());

    // Approve Tint to pull the deposit.
    let approve_call = IERC20::approveCall {
        spender: tint_address,
        amount: U256::MAX,
    };
    send(&eth_provider, token_address, approve_call.abi_encode()).await;

    let mut rng = StdRng::seed_from_u64(provider::DEV_SETUP_SEED);
    let (proving_key, verifying_key) = provider::setup(&mut rng).unwrap();

    let sender_keys = Keys::from_seed(&[11u8; 32]);
    let recipient_keys = Keys::from_seed(&[22u8; 32]);

    let sender_indexer = make_indexer(tint_address, sender_keys.clone(), &rpc_url).await;
    let mut sender_provider =
        Provider::new(sender_indexer, proving_key.clone(), verifying_key.clone());

    let sender_receiver = Receiver {
        nullifier_pub_key: sender_keys.nullifier_key.pub_key(),
        encryption_pub_key: sender_keys.encryption_key.public_key(),
        spendability_address: Address::ZERO,
        spendability_data: Default::default(),
    };
    let asset = AssetId::from(token_address);
    let deposit_amount = 1_000u128;

    let deposit_call = sender_provider
        .deposit(&sender_receiver, asset, deposit_amount, &mut rng)
        .unwrap();
    send(&eth_provider, tint_address, deposit_call.abi_encode()).await;

    sender_provider.indexer.sync().await.unwrap();
    let spendable = sender_provider.indexer.spendable_notes();
    assert_eq!(spendable.len(), 1, "deposited note should be spendable");
    let input_note = spendable[0].clone();
    assert_eq!(input_note.base.amount, deposit_amount);

    // Transfer half to `recipient_keys`, unshield the other half back to an EOA.
    let recipient_receiver = Receiver {
        nullifier_pub_key: recipient_keys.nullifier_key.pub_key(),
        encryption_pub_key: recipient_keys.encryption_key.public_key(),
        spendability_address: Address::ZERO,
        spendability_data: Default::default(),
    };
    let transfer_amount = 400u128;
    let unshield_amount = deposit_amount - transfer_amount;

    let recipient_balance_before =
        balance_of(&eth_provider, token_address, recipient_address).await;

    let operate_call = sender_provider
        .operate(
            [input_note],
            [(recipient_receiver, asset, transfer_amount)],
            [(recipient_address, asset, unshield_amount)],
            &mut rng,
        )
        .unwrap();
    send(&eth_provider, tint_address, operate_call.abi_encode()).await;

    let recipient_balance_after = balance_of(&eth_provider, token_address, recipient_address).await;
    assert_eq!(
        recipient_balance_after - recipient_balance_before,
        U256::from(unshield_amount)
    );

    // The transfer's recipient should independently discover their new note
    // by syncing their own indexer against the same chain.
    let mut recipient_indexer = make_indexer(tint_address, recipient_keys, &rpc_url).await;
    recipient_indexer.sync().await.unwrap();
    let recipient_spendable = recipient_indexer.spendable_notes();
    assert_eq!(recipient_spendable.len(), 1);
    assert_eq!(recipient_spendable[0].base.amount, transfer_amount);
}

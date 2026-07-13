//! Cross-checks that the public-input vector `Provider::public_inputs`
//! computes locally matches what `Tint.computePublicSignals` computes
//! on-chain for the same operation -- independent of proof generation, so a
//! mismatch (e.g. an asset-encoding bug) shows up as a precise, labeled diff
//! instead of an opaque `InvalidProof` revert.

mod common;

use alloy_node_bindings::Anvil;
use alloy_primitives::{Address, U256, hex};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use ark_std::rand::{SeedableRng, rngs::StdRng};
use common::{IERC20, compute_public_signals, deploy, make_indexer, send};
use tint_rs::{
    circuit::join_split::{N_INPUTS, N_OUTPUTS, N_PUB},
    note::{asset::AssetId, keys::Keys, receiver::Receiver},
    provider::{self, Provider},
};

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn public_signals_match_onchain() {
    let anvil = Anvil::new().spawn();
    let rpc_url = anvil.endpoint();
    let deployer_key = hex::encode(anvil.first_key().to_bytes());
    let deployer_signer: PrivateKeySigner = format!("0x{deployer_key}").parse().unwrap();
    let recipient_address = anvil.addresses()[1];

    let (tint_address, token_address) = deploy(&rpc_url, &deployer_key);

    let eth_provider = ProviderBuilder::new()
        .wallet(deployer_signer)
        .connect_http(rpc_url.parse().unwrap());

    let approve_call = IERC20::approveCall {
        spender: tint_address,
        amount: U256::MAX,
    };
    send(&eth_provider, token_address, approve_call.abi_encode()).await;

    let mut rng = StdRng::seed_from_u64(provider::DEV_SETUP_SEED);
    let (proving_key, verifying_key) = provider::setup(&mut rng).unwrap();

    let sender_keys = Keys::from_seed(&[33u8; 32]);
    let recipient_keys = Keys::from_seed(&[44u8; 32]);

    let sender_indexer = make_indexer(tint_address, sender_keys.clone(), &rpc_url).await;
    let mut sender_provider = Provider::new(sender_indexer, proving_key, verifying_key);

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
    let input_note = sender_provider.indexer.spendable_notes()[0].clone();

    let recipient_receiver = Receiver {
        nullifier_pub_key: recipient_keys.nullifier_key.pub_key(),
        encryption_pub_key: recipient_keys.encryption_key.public_key(),
        spendability_address: Address::ZERO,
        spendability_data: Default::default(),
    };
    let transfer_amount = 400u128;
    let unshield_amount = deposit_amount - transfer_amount;

    let (operation, local_signals) = sender_provider
        .public_inputs(
            [input_note],
            [(recipient_receiver, asset, transfer_amount)],
            [(recipient_address, asset, unshield_amount)],
            &mut rng,
        )
        .unwrap();

    let onchain_signals = compute_public_signals(&eth_provider, tint_address, &operation).await;

    let diff = public_signal_diff(&local_signals, &onchain_signals);
    assert!(
        diff.is_empty(),
        "public signal mismatch:\n{}",
        diff.join("\n")
    );
}

/// Compares a locally-computed public-input vector (from
/// `Provider::public_inputs`) against the on-chain one (from
/// `Tint.computePublicSignals`), returning a labeled description of every
/// index that differs.
fn public_signal_diff(local: &[Fr], onchain: &[U256; N_PUB]) -> Vec<String> {
    assert_eq!(local.len(), N_PUB, "expected exactly N_PUB local signals");
    (0..N_PUB)
        .filter_map(|i| {
            let local = fr_to_u256(local[i]);
            (local != onchain[i]).then(|| {
                format!(
                    "{}: rust={local:#x} solidity={:#x}",
                    public_signal_label(i),
                    onchain[i]
                )
            })
        })
        .collect()
}

/// Human-readable label for public-signal index `i`, matching the order
/// `JoinSplit::synthesize` allocates public inputs in (mirrored by
/// `ProofLib.toPublicSignals` / `Tint.computePublicSignals`).
fn public_signal_label(i: usize) -> String {
    match i {
        0 => "old_root".to_string(),
        1 => "old_root_length".to_string(),
        2 => "start_aggregation_hash".to_string(),
        3 => "bound_params_hash".to_string(),
        4 => "new_root".to_string(),
        5 => "end_aggregation_hash".to_string(),
        i if i < 6 + N_INPUTS => format!("nullifiers[{}]", i - 6),
        i if i < 6 + N_INPUTS + N_OUTPUTS => {
            format!("output_commitment_hashes[{}]", i - 6 - N_INPUTS)
        }
        i => {
            let j = i - 6 - N_INPUTS - N_OUTPUTS;
            if j % 2 == 0 {
                format!("unshield_amounts[{}]", j / 2)
            } else {
                format!("unshield_assets[{}]", j / 2)
            }
        }
    }
}

fn fr_to_u256(fr: Fr) -> U256 {
    U256::from_be_bytes(TryInto::<[u8; 32]>::try_into(fr.into_bigint().to_bytes_be()).unwrap())
}

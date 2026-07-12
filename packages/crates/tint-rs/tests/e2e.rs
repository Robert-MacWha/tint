//! End-to-end shield/transfer/unshield test against a live `anvil` node.
//!
//! Requires `Tint` + a `MockVerifier` + a mintable ERC20 already deployed
//! (see `packages/contracts/script/Deploy.s.sol`), with their addresses
//! passed via `TINT_ADDRESS` / `TOKEN_ADDRESS` env vars, and an `anvil`
//! instance reachable at `RPC_URL` (defaults to `http://localhost:8545`).
//!
//! ```sh
//! anvil &
//! forge script packages/contracts/script/Deploy.s.sol:Deploy \
//!   --rpc-url http://localhost:8545 --broadcast \
//!   --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
//! TINT_ADDRESS=<from output> TOKEN_ADDRESS=<from output> cargo test --release --test e2e
//! ```

use std::sync::Arc;

use alloy_primitives::{Address, U256, address};
use alloy_provider::{Provider as AlloyProvider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use alloy_sol_types::SolCall;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint_rs::{
    abis::tint::Tint,
    circuit::join_split::{K, TREE_DEPTH},
    database::memory::MemoryDatabase,
    indexer::{
        Indexer, merkle_tree::IncrementalMerkleTree, syncer::AlloyRpcSyncer, verifier::RootVerifier,
    },
    note::{asset::AssetId, keys::Keys, receiver::Receiver, withdrawal::Withdrawal},
    provider::{self, Provider},
};

sol! {
    interface IERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function balanceOf(address account) external view returns (uint256);
    }
}

/// Anvil's default account #0 — pre-funded, holds the token supply minted
/// by `Deploy.s.sol` (which is broadcast from this same account).
const DEPLOYER_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
/// Anvil's default account #1, used as the unshield recipient.
const RECIPIENT_ADDRESS: Address = address!("0x70997970C51812dc3A010C7d01b50e0d17dc79C8");

fn env_address(key: &str) -> Address {
    std::env::var(key)
        .unwrap_or_else(|_| panic!("{key} not set — see module docs for setup"))
        .parse()
        .unwrap_or_else(|e| panic!("{key} is not a valid address: {e}"))
}

#[tokio::test]
async fn shield_transfer_unshield() {
    let tint_address = env_address("TINT_ADDRESS");
    let token_address = env_address("TOKEN_ADDRESS");
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());

    let signer: PrivateKeySigner = DEPLOYER_KEY.parse().unwrap();
    let deployer = signer.address();
    let eth_provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.parse().unwrap());

    // Approve Tint to pull the deposit.
    let approve_call = IERC20::approveCall {
        spender: tint_address,
        amount: U256::MAX,
    };
    send(&eth_provider, token_address, approve_call.abi_encode()).await;

    let mut rng = StdRng::seed_from_u64(provider::DEV_SETUP_SEED);
    let (proving_key, _vk) = provider::setup(&mut rng).unwrap();

    let sender_keys = Keys::from_seed(&[11u8; 32]);
    let recipient_keys = Keys::from_seed(&[22u8; 32]);

    let sender_indexer =
        make_indexer(&eth_provider, tint_address, sender_keys.clone(), &rpc_url).await;
    let mut sender_provider = Provider::new(sender_indexer, proving_key.clone());

    let sender_receiver = Receiver {
        nullifier_pub_key: sender_keys.nullifier_key.pub_key(),
        encryption_pub_key: sender_keys.encryption_secret_key.public_key(),
        spendability_address: Address::ZERO,
        spendability_data: Default::default(),
    };
    let asset = AssetId::from(token_address);
    let deposit_amount = 1_000u128;

    let deposit_call = sender_provider.deposit(&sender_receiver, asset, deposit_amount, &mut rng);
    send(&eth_provider, tint_address, deposit_call.abi_encode()).await;

    sender_provider.indexer.sync().await.unwrap();
    let spendable = sender_provider.indexer.spendable_notes();
    assert_eq!(spendable.len(), 1, "deposited note should be spendable");
    let input_note = spendable[0].clone();
    assert_eq!(input_note.amount, deposit_amount);

    // Transfer half to `recipient_keys`, unshield the other half back to an EOA.
    let recipient_receiver = Receiver {
        nullifier_pub_key: recipient_keys.nullifier_key.pub_key(),
        encryption_pub_key: recipient_keys.encryption_secret_key.public_key(),
        spendability_address: Address::ZERO,
        spendability_data: Default::default(),
    };
    let transfer_amount = 400u128;
    let unshield_amount = deposit_amount - transfer_amount;

    let recipient_balance_before =
        balance_of(&eth_provider, token_address, RECIPIENT_ADDRESS).await;

    let operation = sender_provider
        .operate(
            vec![input_note],
            vec![(recipient_receiver, asset, transfer_amount)],
            vec![Withdrawal::new(asset, unshield_amount, RECIPIENT_ADDRESS)],
            &mut rng,
        )
        .unwrap();

    let operate_call = Tint::operateCall {
        operations: vec![operation],
    };
    send(&eth_provider, tint_address, operate_call.abi_encode()).await;

    let recipient_balance_after = balance_of(&eth_provider, token_address, RECIPIENT_ADDRESS).await;
    assert_eq!(
        recipient_balance_after - recipient_balance_before,
        U256::from(unshield_amount)
    );

    // The transfer's recipient should independently discover their new note
    // by syncing their own indexer against the same chain.
    let recipient_indexer =
        make_indexer(&eth_provider, tint_address, recipient_keys, &rpc_url).await;
    let mut recipient_indexer = recipient_indexer;
    recipient_indexer.sync().await.unwrap();
    let recipient_spendable = recipient_indexer.spendable_notes();
    assert_eq!(recipient_spendable.len(), 1);
    assert_eq!(recipient_spendable[0].amount, transfer_amount);

    let _ = deployer; // only used to derive the signer's address above
}

async fn make_indexer(
    eth_provider: &impl AlloyProvider,
    tint_address: Address,
    keys: Keys,
    rpc_url: &str,
) -> Indexer {
    let syncer = AlloyRpcSyncer::new(
        ProviderBuilder::new().connect_http(rpc_url.parse().unwrap()),
        tint_address,
    );
    // Checks that the genesis root (the only one registered before any
    // operate() call) is present on-chain, as a basic liveness check.
    let root = IncrementalMerkleTree::<TREE_DEPTH, K>::new().root();
    let verifier = RootVerifier::new(
        ProviderBuilder::new().connect_http(rpc_url.parse().unwrap()),
        tint_address,
        root,
    );
    let _ = eth_provider;

    Indexer::new(
        Arc::new(syncer),
        Arc::new(verifier),
        Arc::new(MemoryDatabase::default()),
        keys.nullifier_key,
        keys.encryption_secret_key,
    )
}

async fn send(provider: &impl AlloyProvider, to: Address, data: Vec<u8>) {
    use alloy_rpc_types_eth::TransactionRequest;

    let tx = TransactionRequest::default().to(to).input(data.into());
    let receipt = provider
        .send_transaction(tx)
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    assert!(receipt.status(), "transaction reverted: {receipt:?}");
}

async fn balance_of(provider: &impl AlloyProvider, token: Address, account: Address) -> U256 {
    use alloy_rpc_types_eth::TransactionRequest;

    let call = IERC20::balanceOfCall { account };
    let tx = TransactionRequest::default()
        .to(token)
        .input(call.abi_encode().into());
    let result = provider.call(tx).await.unwrap();
    IERC20::balanceOfCall::abi_decode_returns(&result).unwrap()
}

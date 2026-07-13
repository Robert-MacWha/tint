#![allow(dead_code)]

use std::{path::Path, process::Command, sync::Arc};

use alloy_primitives::{Address, U256};
use alloy_provider::{Provider as AlloyProvider, ProviderBuilder};
use alloy_sol_macro::sol;
use alloy_sol_types::SolCall;
use tint_rs::{
    abis::tint::{IPrivacyPool, Tint},
    circuit::join_split::{K, N_PUB, TREE_DEPTH},
    database::memory::MemoryDatabase,
    indexer::{
        Indexer, merkle_tree::IncrementalMerkleTree, syncer::AlloyRpcSyncer, verifier::RootVerifier,
    },
    note::keys::Keys,
};

sol! {
    interface IERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function balanceOf(address account) external view returns (uint256);
    }
}

/// Root of the Foundry project (where `foundry.toml` lives; it points `src`/
/// `script`/`out` at `packages/contracts/...`).
pub const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");

/// Deploys `Tint` + `Groth16Verifier` + `MockToken` via `forge script` against
/// the anvil instance at `rpc_url`, broadcasting from `deployer_key` (a raw
/// hex private key, no `0x` prefix). Returns the `(Tint, MockToken)` addresses.
pub fn deploy(rpc_url: &str, deployer_key: &str) -> (Address, Address) {
    let status = Command::new("forge")
        .args([
            "script",
            "packages/contracts/script/Deploy.s.sol:Deploy",
            "--rpc-url",
            rpc_url,
            "--broadcast",
            "--private-key",
            &format!("0x{deployer_key}"),
        ])
        .current_dir(REPO_ROOT)
        .status()
        .expect("failed to run `forge script` -- is `forge` installed and on $PATH?");
    assert!(status.success(), "forge script deploy failed");

    let broadcast_path = Path::new(REPO_ROOT).join("broadcast/Deploy.s.sol/31337/run-latest.json");
    let broadcast: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&broadcast_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", broadcast_path.display())),
    )
    .unwrap();

    let address_of = |name: &str| -> Address {
        broadcast["transactions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tx| tx["contractName"] == name)
            .and_then(|tx| tx["contractAddress"].as_str())
            .unwrap_or_else(|| panic!("no deployed address found for {name}"))
            .parse()
            .unwrap()
    };

    (address_of("Tint"), address_of("MockToken"))
}

pub async fn make_indexer(tint_address: Address, keys: Keys, rpc_url: &str) -> Indexer {
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

    Indexer::new(
        Arc::new(syncer),
        Arc::new(verifier),
        Arc::new(MemoryDatabase::default()),
        keys.nullifier_key,
        keys.encryption_key,
    )
}

pub async fn send(provider: &impl AlloyProvider, to: Address, data: Vec<u8>) {
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

pub async fn balance_of(provider: &impl AlloyProvider, token: Address, account: Address) -> U256 {
    use alloy_rpc_types_eth::TransactionRequest;

    let call = IERC20::balanceOfCall { account };
    let tx = TransactionRequest::default()
        .to(token)
        .input(call.abi_encode().into());
    let result = provider.call(tx).await.unwrap();
    IERC20::balanceOfCall::abi_decode_returns(&result).unwrap()
}

/// Fetches `Tint.computePublicSignals(op)` -- the contract's own computation
/// of the public-signal vector `op` would need to satisfy -- for comparison
/// against what the client proved against.
pub async fn compute_public_signals(
    provider: &impl AlloyProvider,
    tint: Address,
    op: &IPrivacyPool::Operation,
) -> [U256; N_PUB] {
    use alloy_rpc_types_eth::TransactionRequest;

    let call = Tint::computePublicSignalsCall { op: op.clone() };
    let tx = TransactionRequest::default()
        .to(tint)
        .input(call.abi_encode().into());
    let result = provider.call(tx).await.unwrap();
    Tint::computePublicSignalsCall::abi_decode_returns(&result).unwrap()
}

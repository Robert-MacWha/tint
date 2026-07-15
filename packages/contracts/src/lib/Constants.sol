// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

uint256 constant N_INPUTS = 5;
uint256 constant N_OUTPUTS = 5;
uint256 constant N_WITHDRAWALS = 5;
// old_root, old_root_length, start_aggregation_hash, bound_params_hash,
// new_root, end_aggregation_hash, nullifiers, output_commitment_hashes,
// (withdrawal_amount, withdrawal_asset) interleaved per output slot.
uint256 constant N_PUB = 4 + 2 + N_INPUTS + N_OUTPUTS + 2 * N_WITHDRAWALS;
uint128 constant AGGREGATION_RING_SIZE = 256;

/// @dev BN254 scalar field modulus — public signals fed to the Groth16
/// verifier must be reduced into this range.
uint256 constant BN254_FR_MODULUS = 21888242871839275222246405745257275088548364400416034343698204186575808495617;

/// @dev Root of an empty Merkle tree of depth 8, arity 8 (matching the
/// circuit's TREE_DEPTH/K) — the real Poseidon "zeros" chain, not bytes32(0).
/// Pinned by `indexer::genesis_root::matches_solidity_genesis_root` in the
/// Rust crate; must be kept in sync if TREE_DEPTH/K or the Merkle node hash
/// function ever change.
bytes32 constant GENESIS_ROOT = 0x2dbd30e0c2cc00efed70e3ffff71cc81d7ea473f78dff9da61e4c9adf9c1a2ed;

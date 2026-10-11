// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @dev Maximum depth of the MMR tree.
uint128 constant MMR_MAX_DEPTH = 42;

/// @dev Number of frontier commitments kept in history.
uint128 constant MMR_HISTORY_SIZE = 256;

/// @dev `boundParamsHash`, `operationHash`, `histRanks`, `histDepth`, `histCount`.
uint128 constant N_CONST = 5;
uint128 constant N_INPUTS = 5;
uint128 constant N_OUTPUTS = 5;
uint128 constant N_WITHDRAWALS = 2;

/// @dev Index of the first frontier root. The frontier is the tail of the public-signal
/// vector, zero-padded to `MMR_MAX_DEPTH`.
uint128 constant FRONTIER_OFFSET = N_CONST + 2 * N_INPUTS + N_OUTPUTS + 2 * N_WITHDRAWALS;
uint128 constant N_PUB = FRONTIER_OFFSET + MMR_MAX_DEPTH;

/// @dev Number of public inputs the Groth16 verifier checks after hybrid compression.
uint128 constant N_COMPRESSED_PUB = 3;

/// @dev BN254 scalar field modulus.
uint256 constant BN254_FR_MODULUS = 21888242871839275222246405745257275088548364400416034343698204186575808495617;

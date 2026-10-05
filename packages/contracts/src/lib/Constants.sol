// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @dev Mirrors `LibSkewMmr.MAX_DEPTH`. Declared here because it sizes the
/// public-signal vector, which both the circuit and the verifier depend on.
uint128 constant MMR_MAX_DEPTH = 26;

/// @dev `boundParamsHash`, `operationHash`, `histState`, then the frontier.
uint128 constant N_CONST = 3 + MMR_MAX_DEPTH;
uint128 constant N_INPUTS = 5;
uint128 constant N_OUTPUTS = 5;
uint128 constant N_WITHDRAWALS = 2;
uint128 constant N_PUB = N_CONST + 2 * N_INPUTS + N_OUTPUTS + 2 * N_WITHDRAWALS;

/// @dev Number of public inputs the Groth16 verifier checks after hybrid compression.
uint128 constant N_COMPRESSED_PUB = 3;

/// @dev BN254 scalar field modulus.
uint256 constant BN254_FR_MODULUS = 21888242871839275222246405745257275088548364400416034343698204186575808495617;

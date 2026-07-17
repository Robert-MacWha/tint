// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

uint256 constant N_INPUTS = 5;
uint256 constant N_OUTPUTS = 5;
uint256 constant N_WITHDRAWALS = 2;
uint256 constant N_PUB = 6 + 2 * N_INPUTS + N_OUTPUTS + 2 * N_WITHDRAWALS;
uint128 constant AGGREGATION_RING_SIZE = 256;

/// @dev BN254 scalar field modulus.
uint256 constant BN254_FR_MODULUS = 21888242871839275222246405745257275088548364400416034343698204186575808495617;

/// @dev Root of an empty Merkle tree.
bytes32 constant GENESIS_ROOT = 0x11d527274d6e2924fb91d54a03dee9f0351165cc38ddfcbbcb6289a7e9d6adb8;

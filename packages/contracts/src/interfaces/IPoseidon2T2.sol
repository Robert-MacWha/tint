// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @notice Poseidon2 (t=2, BN254) compression.
interface IPoseidon2T2 {
    function compress(uint256 x, uint256 y, uint256 domainSep) external view returns (uint256);
}

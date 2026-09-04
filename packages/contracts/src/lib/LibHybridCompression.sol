// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @notice Hybrid compression (eprint 2025/1500): reconstructs the
/// `alpha`/`gamma` Groth16 public inputs from a prover-supplied `beta` and
/// the plaintext statement vector `stmt`, so the verifier checks only 3
/// field elements instead of one per element of `stmt`.
///
/// @dev Vendored from https://github.com/Robert-MacWha/ark-hybrid-compression
/// (contracts/src/lib/LibHybridCompression.sol).
library LibHybridCompression {
    function hybridCompression(uint256 beta, uint256[] memory stmt, uint256 field)
        internal
        pure
        returns (uint256 alpha, uint256 gamma)
    {
        require(beta < field);
        alpha = hash(stmt, field);
        uint256 sigma = addmod(alpha, beta, field);
        gamma = uhf(sigma, stmt, field);
    }

    function uhf(uint256 sigma, uint256[] memory x, uint256 field) internal pure returns (uint256 acc) {
        acc = 0;
        for (uint256 i = x.length; i > 0; i--) {
            acc = mulmod(acc, sigma, field);
            acc = addmod(acc, x[i - 1], field);
        }
    }

    function hash(uint256[] memory x, uint256 field) internal pure returns (uint256) {
        return uint256(keccak256(abi.encodePacked(x))) % field;
    }
}

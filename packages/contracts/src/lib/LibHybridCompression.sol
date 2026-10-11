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
        bool valid = true;
        assembly ("memory-safe") {
            let data := add(x, 0x20)
            for { let i := mload(x) } i { i := sub(i, 1) } {
                let xi := mload(add(data, shl(5, sub(i, 1))))
                valid := and(valid, lt(xi, field))
                acc := addmod(mulmod(acc, sigma, field), xi, field)
            }
        }
        require(valid);
    }

    function hash(uint256[] memory x, uint256 field) internal pure returns (uint256 h) {
        assembly ("memory-safe") {
            h := mod(keccak256(add(x, 0x20), shl(5, mload(x))), field)
        }
    }
}

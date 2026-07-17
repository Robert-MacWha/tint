// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Poseidon2T3_BN254} from "@taceo/poseidon2/Poseidon2T3_BN254.sol";

/// @notice Inlinable `internal` mirror of {Poseidon2T3_BN254.compress}, avoiding the
/// DELEGATECALL overhead of the library's `external` entrypoint.
library LibPoseidon2T3_BN254 {
    function compress(uint256 x, uint256 y, uint256 z, uint256 domainSep) internal pure returns (uint256) {
        uint256 prime = Poseidon2T3_BN254.PRIME;
        if (x >= prime || y >= prime || z >= prime) revert Poseidon2T3_BN254.NotInPrimefield();
        uint256[3] memory state = Poseidon2T3_BN254._perm([addmod(x, domainSep, prime), y, z]);
        return addmod(state[0], x, prime);
    }
}

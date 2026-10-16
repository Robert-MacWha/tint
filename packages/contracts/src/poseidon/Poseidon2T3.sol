// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Poseidon2T3_BN254} from "@taceo/poseidon2/Poseidon2T3_BN254.sol";
import {IPoseidon2T3} from "../interfaces/IPoseidon2T3.sol";

/// @notice {Poseidon2T3_BN254.compress} as a standalone contract.
contract Poseidon2T3 is IPoseidon2T3 {
    function compress(uint256 x, uint256 y, uint256 z, uint256 domainSep) external pure returns (uint256) {
        uint256 prime = Poseidon2T3_BN254.PRIME;
        if (x >= prime || y >= prime || z >= prime) revert Poseidon2T3_BN254.NotInPrimefield();
        uint256[3] memory state = Poseidon2T3_BN254._perm([addmod(x, domainSep, prime), y, z]);
        return addmod(state[0], x, prime);
    }
}

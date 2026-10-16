// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Poseidon2T2_BN254} from "@taceo/poseidon2/Poseidon2T2_BN254.sol";
import {IPoseidon2T2} from "../interfaces/IPoseidon2T2.sol";

/// @notice {Poseidon2T2_BN254.compress} as a standalone contract.
contract Poseidon2T2 is IPoseidon2T2 {
    function compress(uint256 x, uint256 y, uint256 domainSep) external pure returns (uint256) {
        uint256 prime = Poseidon2T2_BN254.PRIME;
        if (x >= prime || y >= prime) revert Poseidon2T2_BN254.NotInPrimefield();
        uint256[2] memory state = Poseidon2T2_BN254._perm([addmod(x, domainSep, prime), y]);
        return addmod(state[0], x, prime);
    }
}

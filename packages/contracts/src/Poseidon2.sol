// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {LibPoseidon2T2_BN254} from "./lib/LibPoseidon2T2_BN254.sol";
import {LibPoseidon2T3_BN254} from "./lib/LibPoseidon2T3_BN254.sol";

contract Poseidon2 {
    function poseidon2T2(
        uint256[2] calldata inputs
    ) external pure returns (uint256) {
        return LibPoseidon2T2_BN254.compress(inputs[0], inputs[1], 0);
    }

    function poseidon2T3(
        uint256[3] calldata inputs
    ) external pure returns (uint256) {
        return
            LibPoseidon2T3_BN254.compress(inputs[0], inputs[1], inputs[2], 0);
    }
}

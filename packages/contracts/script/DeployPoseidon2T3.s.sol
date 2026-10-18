// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {DeterministicScript} from "./DeterministicScript.sol";

/// Deterministically deploys Poseidon2-T3, or returns it if already deployed. Usage:
///   forge script script/DeployPoseidon2T3.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployPoseidon2T3 is DeterministicScript {
    function run() external returns (Poseidon2T3 poseidon) {
        poseidon = Poseidon2T3(_deploy("Poseidon2T3", type(Poseidon2T3).creationCode));
    }
}

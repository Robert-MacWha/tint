// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {DeterministicScript} from "./DeterministicScript.sol";

/// Deterministically deploys Poseidon2-T2, or returns it if already deployed. Usage:
///   forge script script/DeployPoseidon2T2.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployPoseidon2T2 is DeterministicScript {
    function run() external returns (Poseidon2T2 poseidon) {
        poseidon = Poseidon2T2(_deploy("Poseidon2T2", type(Poseidon2T2).creationCode));
    }
}

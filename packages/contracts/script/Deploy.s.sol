// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {TintVerifier} from "../src/TintVerifier.sol";
import {Tint} from "../src/Tint.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {DeployPoseidon2T3} from "./DeployPoseidon2T3.s.sol";
import {DeterministicScript} from "./DeterministicScript.sol";

/// Deterministically deploys Groth16Verifier and Tint, deploying Poseidon2-T3 first
/// if needed. Already-deployed contracts are reused. Usage:
///   forge script script/Deploy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract Deploy is DeterministicScript {
    function run() external returns (TintVerifier verifier, Tint tint) {
        Poseidon2T3 poseidon = new DeployPoseidon2T3().run();
        verifier = TintVerifier(_deploy("Verifier", type(TintVerifier).creationCode));
        tint = Tint(_deploy("Tint", abi.encodePacked(type(Tint).creationCode, abi.encode(verifier, poseidon))));
    }
}

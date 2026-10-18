// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {TintVerifier} from "../src/TintVerifier.sol";
import {Tint} from "../src/Tint.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {DeployPoseidon2T3} from "./DeployPoseidon2T3.s.sol";
import {SALT} from "./Salt.sol";

/// Deterministically deploys Groth16Verifier and Tint, deploying Poseidon2-T3 first
/// if needed.
///
/// Usage: `forge script script/Deploy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast`
contract Deploy is Script {
    function run() external returns (TintVerifier verifier, Tint tint) {
        Poseidon2T3 poseidon = new DeployPoseidon2T3().run();

        vm.startBroadcast();

        verifier = new TintVerifier{salt: SALT}();
        tint = new Tint{salt: SALT}(verifier, poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("Tint deployed to", address(tint));
    }
}

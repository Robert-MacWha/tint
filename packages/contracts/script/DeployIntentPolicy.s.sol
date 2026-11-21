// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/IntentVerifier.sol";
import {IntentPolicy} from "../src/policy/intent/IntentPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";

/// Deploys the IntentPolicy contract, with its verifier and Poseidon2-T2. Usage:
///   forge script script/DeployIntentPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployIntentPolicy is Script {
    function run() external returns (Verifier verifier, IntentPolicy policy) {
        vm.startBroadcast();

        verifier = new Verifier();
        Poseidon2T2 poseidon = new Poseidon2T2();
        policy = new IntentPolicy(verifier, poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("Poseidon2T2 deployed to", address(poseidon));
        console.log("IntentPolicy deployed to", address(policy));
    }
}

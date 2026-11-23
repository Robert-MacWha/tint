// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/TimelockVerifier.sol";
import {TimelockPolicy} from "../src/policy/timelock/TimelockPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";

/// Deploys the TimelockPolicy contract, with its verifier and Poseidon2-T2. Usage:
///   forge script script/DeployTimelockPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployTimelockPolicy is Script {
    function run() external returns (Verifier verifier, TimelockPolicy policy) {
        vm.startBroadcast();

        Poseidon2T2 poseidon = new Poseidon2T2();
        (verifier, policy) = deployTimelockPolicy(poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("Poseidon2T2 deployed to", address(poseidon));
        console.log("TimelockPolicy deployed to", address(policy));
    }
}

/// Deploys the TimelockPolicy contract with its verifier, over an existing Poseidon2-T2.
function deployTimelockPolicy(Poseidon2T2 poseidon) returns (Verifier verifier, TimelockPolicy policy) {
    verifier = new Verifier();
    policy = new TimelockPolicy(verifier, poseidon);
}

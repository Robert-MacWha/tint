// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/PasswordVerifier.sol";
import {PasswordPolicy} from "../src/policy/password/PasswordPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";

/// Deploys the PasswordPolicy contract, with its verifier and Poseidon2-T2. Usage:
///   forge script script/DeployPasswordPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployPasswordPolicy is Script {
    function run() external returns (Verifier verifier, PasswordPolicy policy) {
        vm.startBroadcast();

        Poseidon2T2 poseidon = new Poseidon2T2();
        (verifier, policy) = deployPasswordPolicy(poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("Poseidon2T2 deployed to", address(poseidon));
        console.log("PasswordPolicy deployed to", address(policy));
    }
}

/// Deploys the PasswordPolicy contract with its verifier, over an existing Poseidon2-T2.
function deployPasswordPolicy(Poseidon2T2 poseidon) returns (Verifier verifier, PasswordPolicy policy) {
    verifier = new Verifier();
    policy = new PasswordPolicy(verifier, poseidon);
}

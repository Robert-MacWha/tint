// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/PasswordVerifier.sol";
import {PasswordPolicy} from "../src/policy/password/PasswordPolicy.sol";

/// Deploys the PasswordPolicy contract. Usage:
///   forge script script/DeployPasswordPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployPasswordPolicy is Script {
    function run() external returns (Verifier verifier, PasswordPolicy policy) {
        vm.startBroadcast();

        verifier = new Verifier();
        policy = new PasswordPolicy(verifier);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("PasswordPolicy deployed to", address(policy));
    }
}

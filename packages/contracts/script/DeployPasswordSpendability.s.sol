// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/PasswordVerifier.sol";
import {PasswordSpendability} from "../src/spendability/password/PasswordSpendability.sol";

/// Deploys the PasswordSpendabilityContract. Usage:
///   forge script script/DeployPasswordSpendability.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployPasswordSpendability is Script {
    function run() external returns (Verifier verifier, PasswordSpendability spendability) {
        vm.startBroadcast();

        verifier = new Verifier();
        spendability = new PasswordSpendability(verifier);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("PasswordSpendability deployed to", address(spendability));
    }
}

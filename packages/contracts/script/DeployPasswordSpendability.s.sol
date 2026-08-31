// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {PasswordSpendabilityVerifier} from "../src/spendability/password/PasswordSpendabilityVerifier.sol";
import {PasswordSpendability} from "../src/spendability/password/PasswordSpendability.sol";

/// Deploys the PasswordSpendabilityContract. Usage:
///   forge script script/DeployPasswordSpendability.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployPasswordSpendability is Script {
    function run() external returns (PasswordSpendabilityVerifier verifier, PasswordSpendability spendability) {
        vm.startBroadcast();

        verifier = new PasswordSpendabilityVerifier();
        spendability = new PasswordSpendability(verifier);

        vm.stopBroadcast();

        console.log("PasswordSpendabilityVerifier deployed to", address(verifier));
        console.log("PasswordSpendability deployed to", address(spendability));
    }
}

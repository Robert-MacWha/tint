// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/MultisigVerifier.sol";
import {MultisigSpendability} from "../src/spendability/multisig/MultisigSpendability.sol";

/// Deploys the MultisigSpendabilityContract. Usage:
///   forge script script/DeployMultisigSpendability.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployMultisigSpendability is Script {
    function run() external returns (Verifier verifier, MultisigSpendability spendability) {
        vm.startBroadcast();

        verifier = new Verifier();
        spendability = new MultisigSpendability(verifier);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("MultisigSpendability deployed to", address(spendability));
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/MultisigVerifier.sol";
import {MultisigPolicy} from "../src/policy/multisig/MultisigPolicy.sol";

/// Deploys the MultisigPolicy contract. Usage:
///   forge script script/DeployMultisigPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployMultisigPolicy is Script {
    function run() external returns (Verifier verifier, MultisigPolicy policy) {
        vm.startBroadcast();

        verifier = new Verifier();
        policy = new MultisigPolicy(verifier);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("MultisigPolicy deployed to", address(policy));
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/MultisigVerifier.sol";
import {MultisigPolicy} from "../src/policy/multisig/MultisigPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";

/// Deploys the MultisigPolicy contract, with its verifier and Poseidon2-T2. Usage:
///   forge script script/DeployMultisigPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployMultisigPolicy is Script {
    function run() external returns (Verifier verifier, MultisigPolicy policy) {
        vm.startBroadcast();

        Poseidon2T2 poseidon = new Poseidon2T2();
        (verifier, policy) = deployMultisigPolicy(poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("Poseidon2T2 deployed to", address(poseidon));
        console.log("MultisigPolicy deployed to", address(policy));
    }
}

/// Deploys the MultisigPolicy contract with its verifier, over an existing Poseidon2-T2.
function deployMultisigPolicy(Poseidon2T2 poseidon) returns (Verifier verifier, MultisigPolicy policy) {
    verifier = new Verifier();
    policy = new MultisigPolicy(verifier, poseidon);
}

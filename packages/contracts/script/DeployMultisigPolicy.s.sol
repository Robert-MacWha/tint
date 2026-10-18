// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/MultisigVerifier.sol";
import {MultisigPolicy} from "../src/policy/multisig/MultisigPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {DeployPoseidon2T2} from "./DeployPoseidon2T2.s.sol";
import {SALT} from "./Salt.sol";

/// Deterministically deploys the MultisigPolicy contract and its verifier, deploying
/// Poseidon2-T2 first if needed.
///
/// Usage: `forge script script/DeployMultisigPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast`
contract DeployMultisigPolicy is Script {
    function run() external returns (Verifier verifier, MultisigPolicy policy) {
        Poseidon2T2 poseidon = new DeployPoseidon2T2().run();

        vm.startBroadcast();

        verifier = new Verifier{salt: SALT}();
        policy = new MultisigPolicy{salt: SALT}(verifier, poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("MultisigPolicy deployed to", address(policy));
    }
}

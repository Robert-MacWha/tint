// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/PasswordVerifier.sol";
import {PasswordPolicy} from "../src/policy/password/PasswordPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {DeployPoseidon2T2} from "./DeployPoseidon2T2.s.sol";
import {SALT} from "./Salt.sol";

/// Deterministically deploys the PasswordPolicy contract and its verifier, deploying
/// Poseidon2-T2 first if needed.
///
/// Usage: `forge script script/DeployPasswordPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast`
contract DeployPasswordPolicy is Script {
    function run() external returns (Verifier verifier, PasswordPolicy policy) {
        Poseidon2T2 poseidon = new DeployPoseidon2T2().run();

        vm.startBroadcast();

        verifier = new Verifier{salt: SALT}();
        policy = new PasswordPolicy{salt: SALT}(verifier, poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("PasswordPolicy deployed to", address(policy));
    }
}

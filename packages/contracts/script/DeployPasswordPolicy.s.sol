// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Verifier} from "../src/codegen/PasswordVerifier.sol";
import {PasswordPolicy} from "../src/policy/password/PasswordPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {DeployPoseidon2T2} from "./DeployPoseidon2T2.s.sol";
import {DeterministicScript} from "./DeterministicScript.sol";

/// Deterministically deploys the PasswordPolicy contract and its verifier, deploying
/// Poseidon2-T2 first if needed. Already-deployed contracts are reused. Usage:
///   forge script script/DeployPasswordPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployPasswordPolicy is DeterministicScript {
    function run() external returns (Verifier verifier, PasswordPolicy policy) {
        Poseidon2T2 poseidon = new DeployPoseidon2T2().run();
        verifier = Verifier(_deploy("Verifier", type(Verifier).creationCode));
        policy = PasswordPolicy(
            _deploy(
                "PasswordPolicy", abi.encodePacked(type(PasswordPolicy).creationCode, abi.encode(verifier, poseidon))
            )
        );
    }
}

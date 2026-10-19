// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Verifier} from "../src/codegen/MultisigVerifier.sol";
import {MultisigPolicy} from "../src/policy/multisig/MultisigPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {DeployPoseidon2T2} from "./DeployPoseidon2T2.s.sol";
import {DeterministicScript} from "./DeterministicScript.sol";

/// Deterministically deploys the MultisigPolicy contract and its verifier, deploying
/// Poseidon2-T2 first if needed. Already-deployed contracts are reused. Usage:
///   forge script script/DeployMultisigPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployMultisigPolicy is DeterministicScript {
    function run() external returns (Verifier verifier, MultisigPolicy policy) {
        Poseidon2T2 poseidon = new DeployPoseidon2T2().run();
        verifier = Verifier(_deploy("Verifier", type(Verifier).creationCode));
        policy = MultisigPolicy(
            _deploy(
                "MultisigPolicy", abi.encodePacked(type(MultisigPolicy).creationCode, abi.encode(verifier, poseidon))
            )
        );
    }
}

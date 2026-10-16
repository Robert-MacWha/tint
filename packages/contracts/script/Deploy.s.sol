// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {TintVerifier} from "../src/TintVerifier.sol";
import {Tint} from "../src/Tint.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";

/// Deploys Groth16Verifier, Poseidon2-T3 and Tint (which takes
/// the verifier's and Poseidon's addresses in its constructor). Usage:
///   forge script script/Deploy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract Deploy is Script {
    function run() external returns (TintVerifier verifier, Tint tint) {
        vm.startBroadcast();

        verifier = new TintVerifier();
        Poseidon2T3 poseidon = new Poseidon2T3();
        tint = new Tint(verifier, poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("Poseidon2T3 deployed to", address(poseidon));
        console.log("Tint deployed to", address(tint));
    }
}

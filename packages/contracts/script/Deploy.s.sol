// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script, console} from "forge-std/Script.sol";
import {Groth16Verifier} from "../src/Groth16Verifier.sol";
import {Tint} from "../src/Tint.sol";
import {AGGREGATION_RING_SIZE} from "../src/lib/Constants.sol";

/// Deploys Groth16Verifier and Tint (which takes the verifier's address in
/// its constructor). Usage:
///   forge script script/Deploy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract Deploy is Script {
    function run() external returns (Groth16Verifier verifier, Tint tint) {
        vm.startBroadcast();

        verifier = new Groth16Verifier();
        tint = new Tint(address(verifier));

        vm.stopBroadcast();

        console.log("Groth16Verifier deployed to", address(verifier));
        console.log("Tint deployed to", address(tint));
    }
}

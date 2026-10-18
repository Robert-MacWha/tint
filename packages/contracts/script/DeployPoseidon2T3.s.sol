// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {SALT} from "./Salt.sol";

/// Deterministically deploys Poseidon2-T3, or returns it if already deployed.
///
/// Usage: `forge script script/DeployPoseidon2T3.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast`
contract DeployPoseidon2T3 is Script {
    function run() external returns (Poseidon2T3 poseidon) {
        poseidon = Poseidon2T3(vm.computeCreate2Address(SALT, keccak256(type(Poseidon2T3).creationCode)));
        if (address(poseidon).code.length > 0) {
            console.log("Poseidon2T3 already deployed at", address(poseidon));
            return poseidon;
        }

        vm.startBroadcast();
        poseidon = new Poseidon2T3{salt: SALT}();
        vm.stopBroadcast();

        console.log("Poseidon2T3 deployed to", address(poseidon));
    }
}

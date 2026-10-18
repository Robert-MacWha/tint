// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {SALT} from "./Salt.sol";

/// Deterministically deploys Poseidon2-T2, or returns it if already deployed.
///
/// Usage: `forge script script/DeployPoseidon2T2.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast`
contract DeployPoseidon2T2 is Script {
    function run() external returns (Poseidon2T2 poseidon) {
        poseidon = Poseidon2T2(vm.computeCreate2Address(SALT, keccak256(type(Poseidon2T2).creationCode)));
        if (address(poseidon).code.length > 0) {
            console.log("Poseidon2T2 already deployed at", address(poseidon));
            return poseidon;
        }

        vm.startBroadcast();
        poseidon = new Poseidon2T2{salt: SALT}();
        vm.stopBroadcast();

        console.log("Poseidon2T2 deployed to", address(poseidon));
    }
}

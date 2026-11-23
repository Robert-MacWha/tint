// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Verifier} from "../src/codegen/HardwareWalletVerifier.sol";
import {HardwareWalletPolicy} from "../src/policy/hardware-wallet/HardwareWalletPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";

/// Deploys the HardwareWalletPolicy contract, with its verifier and Poseidon2-T2. Usage:
///   forge script script/DeployHardwareWalletPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployHardwareWalletPolicy is Script {
    function run() external returns (Verifier verifier, HardwareWalletPolicy policy) {
        vm.startBroadcast();

        Poseidon2T2 poseidon = new Poseidon2T2();
        (verifier, policy) = deployHardwareWalletPolicy(poseidon);

        vm.stopBroadcast();

        console.log("Verifier deployed to", address(verifier));
        console.log("Poseidon2T2 deployed to", address(poseidon));
        console.log("HardwareWalletPolicy deployed to", address(policy));
    }
}

/// Deploys the HardwareWalletPolicy contract with its verifier, over an existing Poseidon2-T2.
function deployHardwareWalletPolicy(Poseidon2T2 poseidon) returns (Verifier verifier, HardwareWalletPolicy policy) {
    verifier = new Verifier();
    policy = new HardwareWalletPolicy(verifier, poseidon);
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";

/// Base for scripts that deploy through the CREATE2 factory, so each contract lands at the same address on every chain.
abstract contract DeterministicScript is Script {
    bytes32 internal constant SALT = keccak256("tint");

    /// Deploys `initCode` via CREATE2, or returns the existing deployment if one is already at its address.
    function _deploy(string memory name, bytes memory initCode) internal returns (address deployed) {
        deployed = vm.computeCreate2Address(SALT, keccak256(initCode));
        if (deployed.code.length > 0) {
            console.log(string.concat(name, " already deployed at"), deployed);
            return deployed;
        }

        vm.broadcast();
        (bool ok,) = CREATE2_FACTORY.call(abi.encodePacked(SALT, initCode));
        require(ok && deployed.code.length > 0, string.concat(name, " deployment failed"));

        console.log(string.concat(name, " deployed to"), deployed);
    }
}

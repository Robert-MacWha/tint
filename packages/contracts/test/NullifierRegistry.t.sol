// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {NullifierRegistry} from "../src/NullifierRegistry.sol";

contract NullifierRegistryHarness is NullifierRegistry {
    function requireUnspent(bytes32 hash) public view {
        _requireUnspent(hash);
    }

    function spend(bytes32 hash) public {
        _spend(hash);
    }
}

contract NullifierRegistryTest is Test {
    NullifierRegistryHarness internal registry;

    function setUp() public {
        registry = new NullifierRegistryHarness();
    }

    function check_spend() public {
        bytes32 nullifier = bytes32(uint256(1));
        registry.spend(nullifier);
        assertEq(registry.isSpent(nullifier), true);
    }

    function check_requireNoDoubleSpend() public {
        bytes32 nullifier = bytes32(uint256(1));
        registry.spend(nullifier);

        vm.expectRevert(abi.encodeWithSelector(NullifierRegistry.NullifierAlreadySpent.selector, nullifier));
        registry.spend(nullifier);
    }

    function check_requireNoopOnZero() public {
        bytes32 nullifier = bytes32(uint256(0));

        registry.spend(nullifier);
        assertEq(registry.isSpent(nullifier), false);
    }
}

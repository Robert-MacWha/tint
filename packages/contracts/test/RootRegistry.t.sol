// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {RootRegistry} from "../src/RootRegistry.sol";

contract RootRegistryHarness is RootRegistry {
    function validateOldRoot(bytes32 root) external view {
        _validateOldRoot(root);
    }

    function updateRoot(bytes32 oldRoot, bytes32 newRoot) external {
        _updateRoot(oldRoot, newRoot);
    }
}

contract RootRegistryTests is Test {
    RootRegistryHarness reg;

    bytes32 constant ROOT_A = bytes32(uint256(0xa));
    bytes32 constant ROOT_B = bytes32(uint256(0xb));
    bytes32 constant ROOT_C = bytes32(uint256(0xc));

    function setUp() public {
        reg = new RootRegistryHarness();
    }

    // ------- constructor -------

    function test_constructor_genesisRootRegistered() public view {
        assertEq(reg.roots(bytes32(0)), 1);
        assertEq(reg.currentRootIndex(), 1);
    }

    // ------- validateOldRoot -------

    function test_validateOldRoot_genesisPasses() public view {
        reg.validateOldRoot(bytes32(0)); // should not revert
    }

    function test_validateOldRoot_unknownRoot_reverts() public {
        vm.expectRevert(RootRegistry.InvalidOldRoot.selector);
        reg.validateOldRoot(ROOT_A);
    }

    function test_validateOldRoot_registeredRootPasses() public {
        reg.updateRoot(bytes32(0), ROOT_A);
        reg.validateOldRoot(ROOT_A); // should not revert
    }

    // ------- updateRoot -------

    function test_updateRoot_advancesIndex() public {
        reg.updateRoot(bytes32(0), ROOT_A); // roots[0]=1, newIdx=2 > 1
        assertEq(reg.currentRootIndex(), 2);
    }

    function test_updateRoot_registersNewRoot() public {
        reg.updateRoot(bytes32(0), ROOT_A);
        assertEq(reg.roots(ROOT_A), 2);
    }

    function test_updateRoot_monotonic() public {
        reg.updateRoot(bytes32(0), ROOT_A); // currentRootIndex=2
        reg.updateRoot(bytes32(0), ROOT_B); // roots[0]=1, newIdx=2, 2 not > 2 → no update
        assertEq(reg.currentRootIndex(), 2); // unchanged
        assertEq(reg.roots(ROOT_B), 0);      // not registered
    }

    function test_updateRoot_doesNotRegisterWhenNotAdvancing() public {
        reg.updateRoot(bytes32(0), ROOT_A); // currentRootIndex=2
        reg.updateRoot(bytes32(0), ROOT_B); // monotonic guard fails
        assertEq(reg.roots(ROOT_B), 0);
    }

    function test_updateRoot_chain() public {
        reg.updateRoot(bytes32(0), ROOT_A); // 0→A: currentRootIndex=2, roots[A]=2
        reg.updateRoot(ROOT_A, ROOT_B);     // A→B: roots[A]=2, newIdx=3 > 2
        assertEq(reg.currentRootIndex(), 3);
        assertEq(reg.roots(ROOT_B), 3);
    }

    function test_updateRoot_chainThree() public {
        reg.updateRoot(bytes32(0), ROOT_A);
        reg.updateRoot(ROOT_A, ROOT_B);
        reg.updateRoot(ROOT_B, ROOT_C); // roots[B]=3, newIdx=4 > 3
        assertEq(reg.currentRootIndex(), 4);
        assertEq(reg.roots(ROOT_C), 4);
    }
}

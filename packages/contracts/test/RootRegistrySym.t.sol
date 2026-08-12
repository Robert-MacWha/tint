// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {SymTest} from "halmos-cheatcodes/SymTest.sol";
import {Test} from "forge-std/Test.sol";
import {RootRegistry} from "../src/RootRegistry.sol";

contract RootRegistryHarness is RootRegistry {
    constructor(bytes32 genesisRoot) RootRegistry(genesisRoot) {}

    function setState(uint128 _tip, bytes32 root, uint128 idx) public {
        currentRootIndex = _tip;
        roots[root] = idx;
    }

    function validateOldRoot(bytes32 oldRoot) public view {
        _validateOldRoot(oldRoot);
    }

    function updateRoot(bytes32 oldRoot, bytes32 newRoot) public {
        _updateRoot(oldRoot, newRoot);
    }
}

contract RootRegistrySymTest is Test, SymTest {
    RootRegistryHarness registry;
    bytes32 genesisRoot;

    function setUp() public {
        genesisRoot = svm.createBytes32("genesisRoot");
        registry = new RootRegistryHarness(genesisRoot);
    }

    /// @dev SAFETY: an arbitrary reachable state, observed through one root.
    ///   - tip >= 1: the constructor sets 1 and _updateRoot only increases it.
    ///   - idx <= tip: every registered root was recorded at the tip.
    ///   - tip < max: 2**128 roots is not physically reachable.
    function _arbitraryState(bytes32 root, uint128 tip, uint128 idx) internal {
        vm.assume(tip >= 1);
        vm.assume(idx <= tip);
        vm.assume(tip < type(uint128).max);
        registry.setState(tip, root, idx);
    }

    /// updateRoot either advances the tip and records the new root, or is a no-op
    /// if oldRoot is not the current tip / if newRoot is not new.
    function check_updateRoot(
        bytes32 oldRoot,
        bytes32 newRoot,
        uint128 tip,
        uint128 oldIdx
    ) public {
        _arbitraryState(oldRoot, tip, oldIdx);

        bool isOldRootTip = registry.roots(oldRoot) == tip;
        registry.updateRoot(oldRoot, newRoot);
        if (isOldRootTip && oldRoot != newRoot) {
            assert(registry.currentRootIndex() == tip + 1);
            assert(registry.roots(oldRoot) == tip);
            assert(registry.roots(newRoot) == tip + 1);
        } else {
            //? Advancing from anything but the tip is a no-op.
            assert(registry.currentRootIndex() == tip);
        }
    }

    /// Check that the tip is never zero.
    function check_tipNeverZero(
        bytes32 oldRoot,
        bytes32 newRoot,
        uint128 tip,
        uint128 oldIdx
    ) public {
        _arbitraryState(oldRoot, tip, oldIdx);
        registry.updateRoot(oldRoot, newRoot);
        assert(registry.currentRootIndex() > 0);
    }

    /// Check that validateOldRoot reverts if the root is not registered.
    function check_validateOldRootRevertsIfNotRegistered(
        bytes32 root,
        bytes32 probe,
        uint128 tip,
        uint128 idx
    ) public {
        _arbitraryState(root, tip, idx);

        // SAFETY: probe must not be the genesis root, which is always registered.
        vm.assume(probe != genesisRoot);

        // SAFETY: idx==0 is the magic number for "unregistered root", so we must not generate it.
        vm.assume(idx != 0);

        try registry.validateOldRoot(probe) {
            if (root != probe) {
                //? If the probe and root are different, the probe was not registered
                // and validateOldRoot should have reverted.
                assert(false);
            }
        } catch {
            if (root == probe) {
                //? If the probe and root are the same, the probe was registered
                // and validateOldRoot should not have reverted.
                assert(false);
            }
        }
    }
}

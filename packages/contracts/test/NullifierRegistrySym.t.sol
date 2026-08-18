// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {SymTest} from "halmos-cheatcodes/SymTest.sol";
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

contract NullifierRegistrySymTest is Test, SymTest {
    NullifierRegistryHarness internal registry;

    function setUp() public {
        registry = new NullifierRegistryHarness();
    }

    /// Checks that spending a nullifier marks it as spent and that it cannot be double-spent.
    function check_spend(bytes32 hash, bytes32 probe, bool preSpend) public {
        vm.assume(hash != bytes32(0));
        vm.assume(probe != bytes32(0));

        if (preSpend) {
            try registry.spend(probe) {} catch {
                assert(false);
            }
        }
        bool before = registry.isSpent(probe);

        vm.assume(probe != hash);
        try registry.spend(hash) {} catch {
            assert(false);
        }
        assert(registry.isSpent(probe) == before);
        assert(registry.isSpent(hash) == (hash != bytes32(0)));
    }

    /// Checks that spent nullifiers cannot be double-spent.
    function check_cannotDoubleSpend(bytes32 hash) public {
        vm.assume(hash != bytes32(0));

        try registry.spend(hash) {} catch {
            assert(false);
        }
        try registry.spend(hash) {
            assert(false);
        } catch {}
    }

    /// Checks that the zero nullifier is a no-op.
    function check_spendZeroIsNoOp() public {
        try registry.spend(bytes32(0)) {} catch {
            assert(false);
        }
        try registry.spend(bytes32(0)) {} catch {
            assert(false);
        }
        try registry.requireUnspent(bytes32(0)) {} catch {
            assert(false);
        }
        assert(registry.isSpent(bytes32(0)) == false);
    }
}

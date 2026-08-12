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

    /// Checks that nullifiers are unspent by default.
    function check_unspentIsUnspent(bytes32 hash) public view {
        vm.assume(hash != bytes32(0));

        try registry.requireUnspent(hash) {} catch {
            assert(false);
        }
    }

    /// Checks that spent nullifiers are no longer unspent.
    function check_spentIsNotUnspent(bytes32 hash) public {
        vm.assume(hash != bytes32(0));

        try registry.spend(hash) {} catch {
            assert(false);
        }
        try registry.requireUnspent(hash) {
            assert(false);
        } catch {}
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
        assert(registry.nullifierHashes(bytes32(0)) == false);
    }

    /// Checks that spending one nullifier does not affect the status of another nullifier.
    function check_spendPreservesOthers(
        bytes32 hash,
        bytes32 probe,
        bool preSpend
    ) public {
        if (preSpend) registry.spend(probe);
        bool before = registry.nullifierHashes(probe);

        vm.assume(probe != hash);
        registry.spend(hash);

        assert(registry.nullifierHashes(probe) == before);
        assert(registry.nullifierHashes(hash) == (hash != bytes32(0)));
    }
}

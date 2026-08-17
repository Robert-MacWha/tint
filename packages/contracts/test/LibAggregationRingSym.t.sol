// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {SymTest} from "halmos-cheatcodes/SymTest.sol";
import {Test} from "forge-std/Test.sol";
import {LibCircularBuffer} from "../src/lib/LibCircularBuffer.sol";
import {LibAggregationRing} from "../src/lib/LibAggregationRing.sol";
import {LibCircularBufferInvariants} from "./LibCircularBufferSym.t.sol";
import {AGGREGATION_RING_SIZE} from "../src/lib/Constants.sol";

contract AggregationRingHarness {
    LibAggregationRing.AggregationRing ring;

    constructor(LibCircularBuffer.CircularBuffer memory _buf) {
        ring.buffer = _buf;
    }

    function setRoot(uint128 index, bytes32 root) public {
        ring.roots[index] = root;
    }

    function stage(bytes32 value) public {
        LibAggregationRing.stage(ring, _hashFn, value);
    }

    function advance(uint128 newTail, bytes32 newRoot) public {
        LibAggregationRing.advance(ring, newTail, newRoot);
    }

    function hashes(uint128 index) public view returns (bytes32) {
        return LibAggregationRing.getHash(ring, index);
    }

    function roots(uint128 index) public view returns (bytes32) {
        return LibAggregationRing.getRoot(ring, index);
    }

    function head() public view returns (uint128) {
        return ring.buffer.head;
    }

    function tail() public view returns (uint128) {
        return ring.buffer.tail;
    }

    function space() public view returns (uint128) {
        return LibCircularBuffer.space(ring.buffer);
    }

    function buffer()
        public
        view
        returns (LibCircularBuffer.CircularBuffer memory)
    {
        return ring.buffer;
    }

    function _hashFn(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(a, b));
    }
}

contract LibAggregationRingSymTest is LibCircularBufferInvariants, SymTest {
    AggregationRingHarness harness;

    /// Assumes an arbitrary reachable state for the circular buffer.
    function _assumeState(
        LibCircularBuffer.CircularBuffer memory _buf
    ) internal {
        // SAFETY 001: Assumes circular buffer is valid.
        _assumeCircularBufferState(_buf);

        harness = new AggregationRingHarness(_buf);

        // SAFETY 002: The root at the tail index must be non-zero.
        bytes32 tailRoot = svm.createBytes32("tailRootValue");
        vm.assume(tailRoot != bytes32(0));
        harness.setRoot(_buf.tail, tailRoot);

        // STATE: Assumes a random root at a random index so the aggregation ring is not empty.
        bytes32 randomRoot = svm.createBytes32("randomRootValue");
        vm.assume(randomRoot != bytes32(0));
        harness.setRoot(
            uint128(svm.createUint(128, "randomRootIndex")),
            randomRoot
        );
    }

    /// Asserts the aggregation ring's invariants. Should be called after every operation.
    function _assertInvariants() internal view {
        // SAFETY 001
        _assertCircularBufferInvariants(harness.buffer());

        // SAFETY 002
        uint128 tail = harness.buffer().tail;
        try harness.roots(tail) returns (bytes32 root) {
            assert(root != bytes32(0));
        } catch {
            // Must never fail
            assert(false);
        }
    }

    /// Checks that staging a value in the aggregation ring produces the correct hash and does not modify the roots.
    function check_stage(
        LibCircularBuffer.CircularBuffer memory _buf,
        bytes32 value,
        uint128 rootProbe
    ) public {
        _assumeState(_buf);

        vm.assume(value != bytes32(0));

        bytes32 rootBefore = harness.roots(rootProbe);
        uint128 headBefore = harness.head();

        try harness.stage(value) {} catch {
            assert(harness.space() == 0);
            return;
        }
        assertEq(harness.roots(rootProbe), rootBefore);

        bytes32 prevHash = headBefore == 0
            ? bytes32(0)
            : harness.hashes(headBefore - 1);
        assertEq(
            harness.hashes(harness.head() - 1),
            keccak256(abi.encodePacked(prevHash, value))
        );

        _assertInvariants();
    }

    /// Checks that staging the magic value 0 is a no-op
    function check_stageMagicValue(
        LibCircularBuffer.CircularBuffer memory _buf,
        uint128 rootProbe
    ) public {
        _assumeState(_buf);

        bytes32 rootBefore = harness.roots(rootProbe);
        uint128 headBefore = harness.head();

        try harness.stage(bytes32(0)) {} catch {
            assert(false);
        }
        assertEq(harness.roots(rootProbe), rootBefore);
        assertEq(harness.head(), headBefore);

        _assertInvariants();
    }

    /// Checks that advancing the tail records the new root, and that a no-op
    /// advance (newTail not past the current tail) changes nothing.
    function check_advance(
        LibCircularBuffer.CircularBuffer memory _buf,
        uint128 newTail,
        bytes32 newRoot,
        uint128 rootProbe
    ) public {
        _assumeState(_buf);

        uint128 headBefore = harness.head();
        uint128 tailBefore = harness.tail();
        bytes32 rootProbeBefore = harness.roots(rootProbe);

        try harness.advance(newTail, newRoot) {} catch {
            // Reverts for either zero root or out-of-bounds tail.
            assert(newTail > tailBefore);
            assert(newRoot == bytes32(0) || newTail > headBefore);
            return;
        }
        if (newTail <= tailBefore) {
            // No-op
            assertEq(harness.tail(), tailBefore);
            assertEq(harness.roots(rootProbe), rootProbeBefore);
        } else {
            // New root recorded
            assertEq(harness.tail(), newTail);
            assertEq(harness.roots(newTail), newRoot);
            if (rootProbe != newTail) {
                assertEq(harness.roots(rootProbe), rootProbeBefore);
            }
        }

        _assertInvariants();
    }
}

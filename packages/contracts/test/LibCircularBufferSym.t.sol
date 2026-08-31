// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {LibCircularBuffer} from "../src/lib/LibCircularBuffer.sol";

contract CircularBufferHarness {
    LibCircularBuffer.CircularBuffer buf;

    constructor(LibCircularBuffer.CircularBuffer memory _buf) {
        buf = _buf;
    }

    function requireSpace(uint128 n) public view {
        LibCircularBuffer.requireSpace(buf, n);
    }

    function space() public view returns (uint128) {
        return LibCircularBuffer.space(buf);
    }

    function push(bytes32 value) public {
        LibCircularBuffer.push(buf, value);
    }

    function get(uint128 index) public view returns (bytes32) {
        return LibCircularBuffer.get(buf, index);
    }

    function requireAdvancable(uint128 n) public view {
        LibCircularBuffer.requireAdvancable(buf, n);
    }

    function advanceTail(uint128 n) public {
        LibCircularBuffer.advanceTail(buf, n);
    }

    function head() public view returns (uint128) {
        return buf.head;
    }

    function tail() public view returns (uint128) {
        return buf.tail;
    }

    function buffer() public view returns (LibCircularBuffer.CircularBuffer memory) {
        return buf;
    }
}

contract LibCircularBufferInvariants is Test {
    /// Assumes an arbitrary reachable state for the circular buffer.
    function _assumeCircularBufferState(LibCircularBuffer.CircularBuffer memory _buf) internal pure {
        // SAFETY 001: Avoids uint128 overflow false positives.
        vm.assume(_buf.head < type(uint16).max);

        // SAFETY 002: The head must be greater than or equal to the tail.
        // Verified in assertInvariants().
        vm.assume(_buf.head >= _buf.tail);

        // SAFETY 003: The head must be at most _buf.buffer.length greater than
        // the tail. Verified in assertInvariants().
        vm.assume(_buf.head < _buf.tail + _buf.buffer.length);
    }

    /// Asserts the circular buffer's invariants. Should be called after every operation.
    function _assertCircularBufferInvariants(LibCircularBuffer.CircularBuffer memory buffer) internal pure {
        // SAFETY 002
        assert(buffer.head >= buffer.tail);

        // SAFETY 003
        assert(buffer.head < buffer.tail + buffer.buffer.length);
    }
}

contract LibCircularBufferSymTest is LibCircularBufferInvariants {
    CircularBufferHarness harness;

    /// Sets the state to an arbitrary assumed state.
    function _setState(LibCircularBuffer.CircularBuffer memory _buf) internal {
        _assumeCircularBufferState(_buf);
        harness = new CircularBufferHarness(_buf);
    }

    /// Checks that pushing a value to the circular buffer behaves correctly.
    function check_push(LibCircularBuffer.CircularBuffer memory _buf, bytes32 value) public {
        _setState(_buf);

        uint128 headBefore = harness.head();
        uint128 tailBefore = harness.tail();
        uint128 spaceBefore = harness.space();

        try harness.push(value) {}
        catch {
            // If the push fails, it must be because the buffer is full.
            assertEq(spaceBefore, 0);
            return;
        }
        // After pushing, following invariants should hold:
        assertEq(harness.head(), headBefore + 1);
        assertEq(harness.tail(), tailBefore);
        assertEq(harness.space(), spaceBefore - 1);

        try harness.get(harness.head() - 1) returns (bytes32 pushedValue) {
            assertEq(pushedValue, value);
        } catch {
            // Must not fail, since we just pushed a value to the buffer.
            assert(false);
        }
        _assertCircularBufferInvariants(harness.buffer());
    }

    /// Checks that `push` never reverts where `requireSpace` succeeded.
    function check_requireSpaceAllowsPush(LibCircularBuffer.CircularBuffer memory _buf, bytes32 value) public {
        _setState(_buf);

        harness.requireSpace(1);
        try harness.push(value) {}
        catch {
            assert(false);
        }
        _assertCircularBufferInvariants(harness.buffer());
    }

    /// Checks that `push` only affects the last value in the circular buffer.
    function check_push_readsAllButLast(LibCircularBuffer.CircularBuffer memory _buf, bytes32 value, uint128 probe)
        public
    {
        _setState(_buf);

        bytes32 probeBefore = harness.get(probe);
        harness.push(value);

        try harness.get(probe) returns (bytes32 probeValue) {
            assertEq(probeValue, probeBefore);
        } catch {
            assert(probe + _buf.buffer.length < harness.head());
        }
        _assertCircularBufferInvariants(harness.buffer());
    }

    /// Checks that advancing the tail of the circular buffer behaves correctly.
    function check_advanceTail(LibCircularBuffer.CircularBuffer memory _buf, uint128 newTail) public {
        _setState(_buf);

        uint128 headBefore = harness.head();

        try harness.advanceTail(newTail) {}
        catch {
            // Failiures must be because the new tail is out of bounds.
            assert(newTail < harness.tail() || newTail > harness.head());
            return;
        }
        assertEq(harness.head(), headBefore);
        assertEq(harness.tail(), newTail);

        _assertCircularBufferInvariants(harness.buffer());
    }

    /// Checks that `advanceTail` never reverts where `requireAdvancable` passed.
    function check_requireAdvancableAllowsAdvance(LibCircularBuffer.CircularBuffer memory _buf, uint128 newTail)
        public
    {
        _setState(_buf);

        harness.requireAdvancable(newTail);
        try harness.advanceTail(newTail) {}
        catch {
            assert(false);
        }
        _assertCircularBufferInvariants(harness.buffer());
    }

    /// Checks that `advanceTail` does not affect the gettable values in the circular buffer.
    function check_advanceTail_readsPastTail(
        LibCircularBuffer.CircularBuffer memory _buf,
        uint128 newTail,
        uint128 probe
    ) public {
        _setState(_buf);

        bytes32 probeBefore = harness.get(probe);
        harness.advanceTail(newTail);

        try harness.get(probe) returns (bytes32 probeValue) {
            assertEq(probeValue, probeBefore);
        } catch {
            assert(false);
        }
        _assertCircularBufferInvariants(harness.buffer());
    }

    /// Checks that getting a value from the circular buffer behaves correctly.
    function check_get(LibCircularBuffer.CircularBuffer memory _buf, uint128 probe) public {
        _setState(_buf);

        try harness.get(probe) {}
        catch {
            assert(probe >= harness.head() || probe + _buf.buffer.length < harness.head());
            return;
        }
        assertLt(probe, harness.head());
        assertGe(probe + _buf.buffer.length, harness.head());

        _assertCircularBufferInvariants(harness.buffer());
    }

    /// Checks that two gets to the same physical slot in the circular buffer cannot both succeed.
    function check_get_no_aliasing(LibCircularBuffer.CircularBuffer memory _buf, uint128 probe1, uint128 probe2)
        public
    {
        _setState(_buf);

        vm.assume(probe1 != probe2);
        // Assumes the two probes point to the same physical slot.
        vm.assume(probe1 % _buf.buffer.length == probe2 % _buf.buffer.length);

        bool valid1 = true;
        try harness.get(probe1) {}
        catch {
            valid1 = false;
        }
        bool valid2 = true;
        try harness.get(probe2) {}
        catch {
            valid2 = false;
        }
        assertFalse(valid1 && valid2);

        _assertCircularBufferInvariants(harness.buffer());
    }
}

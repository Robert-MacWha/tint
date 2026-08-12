// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {SymTest} from "halmos-cheatcodes/SymTest.sol";
import {Test} from "forge-std/Test.sol";
import {AggregationRing} from "../src/AggregationRing.sol";
import {AGGREGATION_RING_SIZE} from "../src/lib/Constants.sol";

contract AggregationRingHarness is AggregationRing {
    function setCounters(uint128 _consumed, uint128 _staged) public {
        consumed = _consumed;
        staged = _staged;
    }

    function commit(bytes32 c) public {
        _commit(c);
    }

    function advanceConsumed(uint128 idx) public {
        _advanceConsumed(idx);
    }

    function getHash(uint128 idx) public view returns (bytes32) {
        return _getHash(idx);
    }

    /// @dev Override poseidon2 with cheaper keccak256. The specifics of the hash function
    /// are irrelevant to the properties being tested.
    function _hash(
        bytes32 prevHash,
        bytes32 commitment
    ) internal pure override returns (bytes32) {
        return keccak256(abi.encode(prevHash, commitment));
    }
}

contract AggregationRingSymTest is Test, SymTest {
    AggregationRingHarness ring;

    function setUp() public {
        ring = new AggregationRingHarness();
    }

    /// @dev SAFETY: Produces an arbitrary reachable state of the ring.
    function _arbitraryState()
        internal
        returns (uint128 consumed, uint128 staged)
    {
        consumed = uint128(svm.createUint(128, "consumed"));
        staged = uint128(svm.createUint(128, "staged"));

        // SAFETY: Enforced by _commit's capacity guard. Tested by check_commit,
        // which ensures that the guard is never violated.
        vm.assume(staged <= AGGREGATION_RING_SIZE);

        // SAFETY: Overflows are not physically reachable, so ruling them out
        // avoids overflow panics as false positives.
        vm.assume(uint256(consumed) + uint256(staged) < type(uint128).max);

        ring.setCounters(consumed, staged);
    }

    /// Succeeds when there is room, and then stages exactly one.
    function check_commit(bytes32 commitment) public {
        (uint128 consumed0, uint128 staged0) = _arbitraryState();

        // SAFETY: Committing the magic value 0 is a no-op.
        vm.assume(commitment != 0);

        try ring.commit(commitment) {
            assert(staged0 < AGGREGATION_RING_SIZE);
            assert(ring.staged() == staged0 + 1);
            assert(ring.consumed() == consumed0);
        } catch {
            //? Only legal failure is when the ring is full.
            assert(staged0 >= AGGREGATION_RING_SIZE);
        }
    }

    /// Committing twice from an arbitrary position extends the hash chain,
    /// including across wraparounds.
    function check_commitExtendsChain(bytes32 c1, bytes32 c2) public {
        (uint128 consumed0, uint128 staged0) = _arbitraryState();

        // SAFETY: Ensure there is room for two more commitments.
        vm.assume(staged0 + 2 <= AGGREGATION_RING_SIZE);
        uint128 total0 = consumed0 + staged0;

        // SAFETY: Ensure the total will not overflow u128.
        vm.assume(uint256(total0) + 2 <= uint256(type(uint128).max));

        // SAFETY: Committing the magic value 0 is a no-op.
        vm.assume(c1 != 0);
        vm.assume(c2 != 0);

        try ring.commit(c1) {} catch {
            //? Cannot fail since we assumed room
            assert(false);
        }
        try ring.getHash(total0 + 1) {} catch {
            //? Cannot fail since we just committed c1
            assert(false);
        }
        bytes32 h1 = ring.getHash(total0 + 1);

        try ring.commit(c2) {} catch {
            //? Cannot fail since we assumed room
            assert(false);
        }
        try ring.getHash(total0 + 2) {} catch {
            //? Cannot fail since we just committed c2
            assert(false);
        }
        bytes32 h2 = ring.getHash(total0 + 2);
        assert(h2 == keccak256(abi.encode(h1, c2)));
    }

    /// Advancing moves commitments from staged into consumed.
    ///
    /// @dev Advancing to an already-consumed index is a no-op, not an error,
    /// to avoid SDK race conditions.
    function check_advanceConsumed(uint128 idx) public {
        (uint128 consumed0, uint128 staged0) = _arbitraryState();

        try ring.advanceConsumed(idx) {
            assert(ring.consumed() + ring.staged() == consumed0 + staged0);
            assert(ring.consumed() >= consumed0);

            if (idx > consumed0) {
                assert(ring.consumed() == idx);
            } else {
                //? Advancing to an already-consumed index is a no-op.
                assert(ring.consumed() == consumed0);
                assert(ring.staged() == staged0);
            }
        } catch {
            // Over-consumption is the only legal failure.
            assert(idx > consumed0 + staged0);
        }
    }

    /// Checks that `_getHash` reverts for indices outside the live window. The
    /// live window is `total - N < idx <= total` where `total = consumed + staged`.
    /// Indices outside that window are either too old (recycled) or too new (don't
    /// exist) and would overflow/underflow the ring buffer.
    function check_getHashAcceptsExactlyLiveIndices(uint128 idx) public {
        (uint128 consumed0, uint128 staged0) = _arbitraryState();
        uint128 total = consumed0 + staged0;

        bool shouldSucceed = idx == 0 ||
            (total >= idx && total - idx < AGGREGATION_RING_SIZE);

        try ring.getHash(idx) {
            assert(shouldSucceed);
        } catch {
            assert(!shouldSucceed);
        }
    }

    /// Check that advancing the consumed pointer does not narrow the live window.
    /// Accessible indices before the advance should remain accessible after the advance.
    function check_advanceDoesNotNarrowWindow(
        uint128 idx,
        uint128 target
    ) public {
        _arbitraryState();
        bytes32 before = ring.getHash(idx);
        ring.advanceConsumed(target);

        try ring.getHash(idx) returns (bytes32 after_) {
            assert(before == after_);
        } catch {
            assert(false);
        }
    }

    /// Check that committing never disturbs the hash at an older index. Older
    /// indices are either still readable and unchanged, or have been recycled
    /// and are no longer readable.
    function check_commitDoesNotDisturbOlderHashes(
        uint128 idx,
        bytes32 commitment
    ) public {
        _arbitraryState();
        bytes32 before = ring.getHash(idx);

        ring.commit(commitment);
        try ring.getHash(idx) returns (bytes32 after_) {
            assert(before == after_);
        } catch {
            //? If the index is no longer readable, it must have been recycled.
            uint128 total = ring.consumed() + ring.staged();
            assert(total - idx >= AGGREGATION_RING_SIZE);
        }
    }
}

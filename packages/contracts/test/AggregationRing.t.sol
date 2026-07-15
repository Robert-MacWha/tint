// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {LibPoseidon2Yul} from "poseidon2-evm/src/bn254/yul/LibPoseidon2Yul.sol";
import {AggregationRing} from "../src/AggregationRing.sol";
import {AGGREGATION_RING_SIZE} from "../src/lib/Constants.sol";

contract AggregationRingHarness is AggregationRing {
    function commit(bytes32 c) external {
        _commit(c);
    }

    function advance(uint128 idx) external {
        _advanceConsumed(idx);
    }

    function validateAndGetHash(uint128 idx) external view returns (bytes32) {
        return _getHash(idx);
    }
}

contract AggregationRingTests is Test {
    AggregationRingHarness ring;

    bytes32 constant C1 = bytes32(uint256(0xc1));
    bytes32 constant C2 = bytes32(uint256(0xc2));
    bytes32 constant C3 = bytes32(uint256(0xc3));

    function setUp() public {
        ring = new AggregationRingHarness();
    }

    // ------- hash correctness -------

    function test_commit_singleHash() public {
        ring.commit(C1);
        bytes32 expected = bytes32(
            LibPoseidon2Yul.hash_2(uint256(0), uint256(C1))
        );
        assertEq(ring.aggregationHashRing(0), expected);
    }

    function test_commit_chainHash() public {
        ring.commit(C1);
        bytes32 h0 = ring.aggregationHashRing(0);
        ring.commit(C2);
        bytes32 expected = bytes32(
            LibPoseidon2Yul.hash_2(uint256(h0), uint256(C2))
        );
        assertEq(ring.aggregationHashRing(1), expected);
    }

    function test_commit_threeChain() public {
        ring.commit(C1);
        ring.commit(C2);
        bytes32 h1 = ring.aggregationHashRing(1);
        ring.commit(C3);
        bytes32 expected = bytes32(
            LibPoseidon2Yul.hash_2(uint256(h1), uint256(C3))
        );
        assertEq(ring.aggregationHashRing(2), expected);
    }

    // ------- counter -------

    function test_commit_incrementsTotalStaged() public {
        assertEq(ring.totalStaged(), 0);
        ring.commit(C1);
        assertEq(ring.totalStaged(), 1);
        ring.commit(C2);
        assertEq(ring.totalStaged(), 2);
    }

    // ------- capacity -------

    function test_commit_full_reverts() public {
        for (uint256 i; i < AGGREGATION_RING_SIZE; ++i) {
            ring.commit(bytes32(uint256(i + 1)));
        }
        vm.expectRevert(AggregationRing.StagingFull.selector);
        ring.commit(bytes32(uint256(AGGREGATION_RING_SIZE + 1)));
    }

    // ------- ring wrapping -------

    function test_commit_ringWraparound() public {
        // Fill ring to capacity
        for (uint256 i; i < AGGREGATION_RING_SIZE; ++i) {
            ring.commit(bytes32(uint256(i + 1)));
        }
        bytes32 hashAtSlot0Before = ring.aggregationHashRing(0);

        // Consume all slots so the ring has room again
        ring.advance(uint128(AGGREGATION_RING_SIZE - 1));

        // Commit at index AGGREGATION_RING_SIZE → writes to slot 0 (wraps)
        bytes32 prevHash = ring.aggregationHashRing(AGGREGATION_RING_SIZE - 1);
        bytes32 newCommitment = bytes32(uint256(AGGREGATION_RING_SIZE + 1));
        ring.commit(newCommitment);

        bytes32 expected = bytes32(
            LibPoseidon2Yul.hash_2(uint256(prevHash), uint256(newCommitment))
        );
        assertEq(ring.aggregationHashRing(0), expected);
        assertFalse(ring.aggregationHashRing(0) == hashAtSlot0Before); // slot was overwritten
    }

    function test_commit_slotRecycledAfterConsume() public {
        // Commit to slots 0..N-1
        for (uint256 i; i < AGGREGATION_RING_SIZE; ++i) {
            ring.commit(bytes32(uint256(i + 1)));
        }
        // Consume first half (slots 0..63 freed)
        ring.advance(63);

        // Ring has room for 64 more (consumed 64, staged 128, diff = 64)
        bytes32 newC = bytes32(uint256(999));
        ring.commit(newC); // writes to slot 128 % 128 = 0

        bytes32 prevHash = ring.aggregationHashRing(AGGREGATION_RING_SIZE - 1);
        bytes32 expected = bytes32(
            LibPoseidon2Yul.hash_2(uint256(prevHash), uint256(newC))
        );
        assertEq(ring.aggregationHashRing(0), expected);
    }

    // ------- consumed-pointer gating -------

    function test_advance_updatesConsumed() public {
        ring.commit(C1);
        ring.commit(C2);
        ring.advance(1);
        assertEq(ring.totalConsumed(), 1);
    }

    function test_advance_doesNotDecrease() public {
        ring.commit(C1);
        ring.commit(C2);
        ring.advance(1);
        ring.advance(0); // idx < totalConsumed → no-op
        assertEq(ring.totalConsumed(), 1);
    }

    function test_getHash() public {
        ring.commit(C1);
        bytes32 expected = ring.aggregationHashRing(0);
        assertEq(ring.validateAndGetHash(0), expected);
    }
}

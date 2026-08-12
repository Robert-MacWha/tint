// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {Poseidon2T2_BN254} from "@taceo/poseidon2/Poseidon2T2_BN254.sol";
import {AggregationRing} from "../src/AggregationRing.sol";
import {AGGREGATION_RING_SIZE} from "../src/lib/Constants.sol";

contract AggregationRingHarness is AggregationRing {
    function commit(bytes32 c) public {
        _commit(c);
    }
}

contract AggregationRingTests is Test {
    AggregationRingHarness ring;

    bytes32 constant C1 = bytes32(uint256(0xc1));
    bytes32 constant C2 = bytes32(uint256(0xc2));

    function setUp() public {
        ring = new AggregationRingHarness();
    }

    /// Test that the aggregation ring correctly computes the Poseidon hash for
    /// the hash chain.
    function test_commit() public {
        ring.commit(C1);
        bytes32 expected = bytes32(
            Poseidon2T2_BN254.compress([uint256(0), uint256(C1)], 0)
        );
        assertEq(ring.aggregationHashRing(0), expected);

        ring.commit(C2);
        expected = bytes32(
            Poseidon2T2_BN254.compress([uint256(expected), uint256(C2)], 0)
        );
        assertEq(ring.aggregationHashRing(1), expected);
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {PoseidonT3} from "poseidon-solidity/PoseidonT3.sol";
import {AGGREGATION_RING_SIZE} from "./lib/Constants.sol";

/// @notice Manages the circular Poseidon hash ring used to batch commitments for zk-proof aggregation.
///
/// Each commitment extends the chain: ring[i % N] = Poseidon(ring[(i-1) % N], commitment).
/// Operations reference a specific ring index; the hash at that index is included as a public
/// input to the zk proof, binding the proof to a concrete set of staged commitments.
contract AggregationRing {
    uint128 public totalStaged; // total commitments ever staged
    uint128 public totalConsumed; // total commitments consumed by operations (packed with totalStaged)
    bytes32[AGGREGATION_RING_SIZE] public aggregationHashRing;

    error StagingFull();
    error InvalidAggregationIndex();

    function _commit(bytes32 commitment) internal {
        if (totalStaged - totalConsumed >= AGGREGATION_RING_SIZE)
            revert StagingFull();

        bytes32 prevHash = totalStaged > 0
            ? aggregationHashRing[(totalStaged - 1) % AGGREGATION_RING_SIZE]
            : bytes32(0);

        aggregationHashRing[totalStaged % AGGREGATION_RING_SIZE] = bytes32(
            PoseidonT3.hash([uint256(prevHash), uint256(commitment)])
        );

        totalStaged++;
    }

    /// Returns the hash at `idx`, or reverts if the index is outside the valid window.
    function _validateAndGetHash(uint128 idx) internal view returns (bytes32) {
        if (idx > totalStaged || idx < totalConsumed)
            revert InvalidAggregationIndex();
        return aggregationHashRing[idx % AGGREGATION_RING_SIZE];
    }

    /// Advances the consumed pointer to idx+1 if idx is not already consumed.
    function _advanceConsumed(uint128 idx) internal {
        if (idx >= totalConsumed) totalConsumed = idx + 1;
    }
}

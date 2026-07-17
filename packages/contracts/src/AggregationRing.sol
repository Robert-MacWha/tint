// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {LibPoseidon2T2_BN254} from "./lib/LibPoseidon2T2_BN254.sol";
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

    event AdvanceAggregationRing(uint128 idx);

    error StagingFull();
    error InvalidAggregationIndex();

    /// Stages a commitment into the aggregation ring, extending the Poseidon hash chain.
    ///
    /// @dev Reverts if the ring is full
    ///
    /// TODO: Rename to _post to avoid confusion with "commitments"
    function _commit(bytes32 commitment) internal {
        if (totalStaged - totalConsumed >= AGGREGATION_RING_SIZE)
            revert StagingFull();

        bytes32 prevHash = totalStaged > 0
            ? aggregationHashRing[(totalStaged - 1) % AGGREGATION_RING_SIZE]
            : bytes32(0);

        aggregationHashRing[totalStaged % AGGREGATION_RING_SIZE] = bytes32(
            LibPoseidon2T2_BN254.compress(uint256(prevHash), uint256(commitment), 0)
        );

        totalStaged++;
    }

    /// Returns the hash after `idx` commitments have been staged (0 if none
    /// have been staged yet).
    ///
    /// TODO: Figure out a better way to handle range errors. Essentially we want to ensure the
    /// idx is properly constrained so a malicious actor can't overwrite or skip commitments.
    function _getHash(uint128 idx) internal view returns (bytes32) {
        if (idx == 0) return bytes32(0);
        if (idx > totalStaged) revert InvalidAggregationIndex();
        return aggregationHashRing[(idx - 1) % AGGREGATION_RING_SIZE];
    }

    /// Advances the consumed pointer to idx if idx is not already consumed.
    function _advanceConsumed(uint128 idx) internal {
        if (idx <= totalConsumed) return;

        totalConsumed = idx;
        emit AdvanceAggregationRing(idx);
    }
}

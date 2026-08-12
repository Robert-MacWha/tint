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
    uint128 public consumed; // current number of consumed commitments
    uint128 public staged; // current number of staged commitments
    bytes32[AGGREGATION_RING_SIZE] public aggregationHashRing;

    event AdvanceAggregationRing(uint128 idx);

    error StagingFull();
    error InvalidAggregationIndex();

    /// Stages a commitment into the aggregation ring, extending the Poseidon hash chain.
    ///
    /// @dev Reverts if the ring is full. No-op if the commitment is the magic value 0.
    function _commit(bytes32 commitment) internal {
        if (commitment == 0) return;
        _requireCapacity(1);

        uint128 total = _total();
        bytes32 prevHash = total > 0
            ? aggregationHashRing[(total - 1) % AGGREGATION_RING_SIZE]
            : bytes32(0);

        aggregationHashRing[total % AGGREGATION_RING_SIZE] = _hash(
            prevHash,
            commitment
        );

        ++staged;
    }

    /// @dev Overridable in test harnesses to swap out the hash function.
    function _hash(
        bytes32 prevHash,
        bytes32 commitment
    ) internal view virtual returns (bytes32) {
        return
            bytes32(
                LibPoseidon2T2_BN254.compress(
                    uint256(prevHash),
                    uint256(commitment),
                    0
                )
            );
    }

    /// Returns the hash after `idx` commitments have been staged (0 if none
    /// have been staged yet).
    function _getHash(uint128 idx) internal view returns (bytes32) {
        if (idx == 0) return bytes32(0);
        _requireLive(idx);
        return aggregationHashRing[(idx - 1) % AGGREGATION_RING_SIZE];
    }

    /// Advances the consumed pointer to idx if idx is not already consumed.
    function _advanceConsumed(uint128 idx) internal {
        if (idx <= consumed) return;
        _requireLive(idx);

        staged -= (idx - consumed);
        consumed = idx;
        emit AdvanceAggregationRing(idx);
    }

    /// @notice Reverts if staging `count` commitments would overflow the ring.
    function _requireCapacity(uint128 count) internal view {
        if (staged + count > AGGREGATION_RING_SIZE) revert StagingFull();
    }

    /// @notice Reverts if idx is outside the live range of the ring (consumed, consumed + AGGREGATION_RING_SIZE].
    function _requireLive(uint128 idx) internal view {
        uint128 total = _total();
        if (total < idx) revert InvalidAggregationIndex();
        if (total - idx >= AGGREGATION_RING_SIZE)
            revert InvalidAggregationIndex();
    }

    /// @notice Returns the total number of consumed and staged commitments in the ring
    function _total() internal view returns (uint128) {
        return consumed + staged;
    }
}

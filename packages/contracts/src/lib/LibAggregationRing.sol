// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {LibCircularBuffer} from "./LibCircularBuffer.sol";
import {AGGREGATION_RING_SIZE, GENESIS_ROOT} from "./Constants.sol";

library LibAggregationRing {
    using LibCircularBuffer for LibCircularBuffer.CircularBuffer;

    struct AggregationRing {
        LibCircularBuffer.CircularBuffer buffer;
        mapping(uint128 index => bytes32 root) roots;
    }

    bytes32 private constant MAGIC_BYTES = 0;

    error MissingRoot(uint128 index);
    error InvalidRoot();

    /// @notice Initializes the aggregation ring with a genesis root and an empty circular buffer.
    function init(AggregationRing storage self, uint256 count) internal {
        self.roots[0] = GENESIS_ROOT;
        LibCircularBuffer.init(self.buffer, count);
    }

    /// @notice Reverts if staging `count` values would overflow the ring.
    function requireSpace(AggregationRing storage self, uint128 count) internal view {
        self.buffer.requireSpace(count);
    }

    /// @notice Returns the number of free slots in the ring.
    function space(AggregationRing storage self) internal view returns (uint128) {
        return self.buffer.space();
    }

    /// @notice Stages a new value to the aggregation ring. Reverts if the ring is full.
    ///
    /// @dev No-op if the value is the magic value 0.
    function stage(
        AggregationRing storage self,
        function(bytes32, bytes32) internal pure returns (bytes32) hashFn,
        bytes32 value
    ) internal {
        if (value == MAGIC_BYTES) return;

        bytes32 prevHash = getHash(self, self.buffer.head);

        bytes32 newHash = hashFn(prevHash, value);
        self.buffer.push(newHash);
    }

    /// @notice Reverts if the aggregation ring cannot be advanced.
    function requireAdvanceable(AggregationRing storage self, uint128 newTail, bytes32 newRoot) internal view {
        self.buffer.requireAdvancable(newTail);
        if (newRoot == MAGIC_BYTES) revert InvalidRoot();
    }

    /// @notice Advances the aggregation ring to a new tail index and records the root hash at that index.
    /// Reverts if the new root is the magic value 0.
    ///
    /// @dev No-op if the new tail is not an advancement.
    function advance(AggregationRing storage self, uint128 newTail, bytes32 newRoot) internal {
        if (newTail <= self.buffer.tail) return;
        requireAdvanceable(self, newTail, newRoot);

        self.buffer.advanceTail(newTail);
        self.roots[newTail] = newRoot;
    }

    /// @notice Returns the hash after `index` values have been staged or 0 if none
    /// have been staged. Reverts if the index is out of bounds.
    function getHash(AggregationRing storage self, uint128 index) internal view returns (bytes32) {
        if (index == 0) return bytes32(0);
        return self.buffer.get(index - 1);
    }

    /// @notice Returns the root hash at a given index. Reverts if no root is recorded at that index.
    function getRoot(AggregationRing storage self, uint128 index) internal view returns (bytes32) {
        bytes32 root = self.roots[index];
        if (root == MAGIC_BYTES) {
            revert MissingRoot(index);
        }
        return root;
    }

    /// @notice Returns the index of the latest root added to the aggregation ring.
    function latestRootIndex(AggregationRing storage self) internal view returns (uint128) {
        return self.buffer.tail;
    }
}

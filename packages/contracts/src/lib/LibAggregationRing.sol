// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {LibCircularBuffer} from "./LibCircularBuffer.sol";
import {GENESIS_ROOT} from "./Constants.sol";

library LibAggregationRing {
    struct AggregationRing {
        LibCircularBuffer.CircularBuffer buffer;
        mapping(uint128 index => bytes32 root) roots;
    }

    bytes32 private constant MAGIC_BYTES = 0;

    error MissingRoot(uint128 index);
    error InvalidRoot();

    /// @notice Initializes the aggregation ring with a genesis root and an empty circular buffer.
    function init(AggregationRing storage self) internal {
        self.roots[0] = GENESIS_ROOT;
        LibCircularBuffer.init(self.buffer);
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

        bytes32 prevHash = self.buffer.head == 0
            ? bytes32(0)
            : LibCircularBuffer.get(self.buffer, self.buffer.head - 1);

        bytes32 newHash = hashFn(prevHash, value);
        LibCircularBuffer.push(self.buffer, newHash);
    }

    /// @notice Advances the aggregation ring to a new tail index and records the root hash at that index.
    /// Reverts if the new root is the magic value 0.
    ///
    /// @dev No-op if the new tail is not greater than the current tail.
    function advance(
        AggregationRing storage self,
        uint128 newTail,
        bytes32 newRoot
    ) internal {
        if (newTail <= self.buffer.tail) return;
        if (newRoot == MAGIC_BYTES) revert InvalidRoot();

        LibCircularBuffer.advanceTail(self.buffer, newTail);
        self.roots[newTail] = newRoot;
    }

    /// @notice Returns the hash at the given index in the aggregation ring. Reverts if the index is out of bounds.
    function getHash(
        AggregationRing storage self,
        uint128 index
    ) internal view returns (bytes32) {
        return LibCircularBuffer.get(self.buffer, index);
    }

    /// @notice Returns the root hash at a given index. Reverts if no root is recorded at that index.
    function getRoot(
        AggregationRing storage self,
        uint128 index
    ) internal view returns (bytes32) {
        bytes32 root = self.roots[index];
        if (root == MAGIC_BYTES) {
            revert MissingRoot(index);
        }
        return root;
    }

    /// @notice Returns the index of the latest root added to the aggregation ring.
    function latestRootIndex(
        AggregationRing storage self
    ) internal view returns (uint128) {
        return self.buffer.tail;
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AGGREGATION_RING_SIZE} from "./Constants.sol";

library LibCircularBuffer {
    struct CircularBuffer {
        uint128 head;
        uint128 tail;
        bytes32[AGGREGATION_RING_SIZE] buffer;
    }

    error CircularBufferFull();
    error OutOfBounds(uint128 index, uint128 tail, uint128 head);

    /// @notice Initializes the circular buffer.
    function init(CircularBuffer storage self) internal {
        self.head = 0;
        self.tail = 0;
    }

    /// @notice Requires that the circular buffer has at least `count` free slots. Reverts if not.
    function requireSpace(
        CircularBuffer storage self,
        uint128 count
    ) internal view {
        if (space(self) < count) {
            revert CircularBufferFull();
        }
    }

    /// @notice Returns the number of free slots in the circular buffer.
    function space(
        CircularBuffer storage self
    ) internal view returns (uint128) {
        return AGGREGATION_RING_SIZE - (self.head - self.tail);
    }

    /// @notice Pushes a value to the circular buffer. Reverts if the buffer is full.
    function push(CircularBuffer storage self, bytes32 value) internal {
        uint128 head = self.head;
        if (AGGREGATION_RING_SIZE <= head - self.tail) {
            revert CircularBufferFull();
        }

        self.buffer[index(head)] = value;
        self.head = head + 1;
    }

    /// @notice Gets a value from the circular buffer at the given index. Reverts if the index is out of bounds.
    function get(
        CircularBuffer storage self,
        uint128 _index
    ) internal view returns (bytes32) {
        if (_index >= self.head) {
            revert OutOfBounds(_index, self.tail, self.head);
        }
        if (_index + AGGREGATION_RING_SIZE < self.head) {
            revert OutOfBounds(_index, self.tail, self.head);
        }
        return self.buffer[index(_index)];
    }

    /// @notice Advances the tail of the circular buffer. Reverts if the new tail is out of bounds.
    function advanceTail(
        CircularBuffer storage self,
        uint128 newTail
    ) internal {
        if (newTail < self.tail || newTail > self.head) {
            revert OutOfBounds(newTail, self.tail, self.head);
        }
        self.tail = newTail;
    }

    /// @notice Returns x as an index in the circular buffer, wrapping if necessary.
    function index(uint128 x) private pure returns (uint128) {
        return x % AGGREGATION_RING_SIZE;
    }
}

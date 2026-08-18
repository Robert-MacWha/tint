// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

library LibCircularBuffer {
    struct CircularBuffer {
        uint128 head;
        uint128 tail;
        bytes32[] buffer;
    }

    error CircularBufferFull();
    error OutOfBounds(uint128 index, uint128 tail, uint128 head);

    /// @notice Initializes the circular buffer.
    function init(CircularBuffer storage self, uint256 count) internal {
        self.buffer = new bytes32[](count);
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
    ///
    /// @dev One slot is always reserved so the value at `tail - 1` (the
    /// staging hash checkpoint for the current tail) is never overwritten
    /// before it can be read via `get`.
    function space(
        CircularBuffer storage self
    ) internal view returns (uint128) {
        return uint128(self.buffer.length) - 1 - (self.head - self.tail);
    }

    /// @notice Pushes a value to the circular buffer. Reverts if the buffer is full.
    function push(CircularBuffer storage self, bytes32 value) internal {
        uint128 head = self.head;
        if (space(self) == 0) {
            revert CircularBufferFull();
        }

        self.buffer[index(self, head)] = value;
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
        if (_index + self.buffer.length < self.head) {
            revert OutOfBounds(_index, self.tail, self.head);
        }
        return self.buffer[index(self, _index)];
    }

    /// @notice Reverts if the tail cannot be advanced to newTail.
    function requireAdvancable(
        CircularBuffer storage self,
        uint128 newTail
    ) internal view {
        if (newTail < self.tail || newTail > self.head) {
            revert OutOfBounds(newTail, self.tail, self.head);
        }
    }

    /// @notice Advances the tail of the circular buffer. Reverts if the new tail is out of bounds.
    function advanceTail(
        CircularBuffer storage self,
        uint128 newTail
    ) internal {
        requireAdvancable(self, newTail);
        self.tail = newTail;
    }

    /// @notice Returns x as an index in the circular buffer, wrapping if necessary.
    function index(
        CircularBuffer storage self,
        uint128 x
    ) private view returns (uint128) {
        return x % uint128(self.buffer.length);
    }
}

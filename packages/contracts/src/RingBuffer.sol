// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice A minimal fixed-capacity ring buffer of uint256 values.
/// When full, pushing a new value overwrites the oldest one.
contract RingBuffer {
    uint256[] private buffer;

    uint256 public immutable capacity;
    uint256 public head;   // next slot to write to
    uint256 public count;  // number of items currently stored (caps at capacity)

    constructor(uint256 _capacity) {
        require(_capacity > 0, "capacity must be > 0");
        capacity = _capacity;
        buffer = new uint256[](_capacity);
    }

    /// @notice Add a value, overwriting the oldest entry if the buffer is full.
    function push(uint256 value) external {
        buffer[head] = value;
        head = (head + 1) % capacity;
        if (count < capacity) {
            count++;
        }
    }

    /// @notice Read the i-th oldest item still in the buffer (0 = oldest).
    function get(uint256 i) external view returns (uint256) {
        require(i < count, "index out of range");
        uint256 start = (head + capacity - count) % capacity;
        return buffer[(start + i) % capacity];
    }

    /// @notice Return all currently stored items, oldest first.
    function getAll() external view returns (uint256[] memory items) {
        items = new uint256[](count);
        uint256 start = (head + capacity - count) % capacity;
        for (uint256 i = 0; i < count; i++) {
            items[i] = buffer[(start + i) % capacity];
        }
    }

    function isFull() external view returns (bool) {
        return count == capacity;
    }
}

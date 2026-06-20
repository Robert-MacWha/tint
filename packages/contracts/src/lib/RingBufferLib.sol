// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

library RingBufferLib {
    struct RingBuffer {
        bytes32[] data;
        uint256 head;
        uint256 count;
        uint256 capacity;
    }

    error NotEnoughItems();
    error IndexOutOfRange();

    function init(RingBuffer storage self, uint256 _capacity) internal {
        self.capacity = _capacity;
        self.data = new bytes32[](_capacity);
    }

    function push(RingBuffer storage self, bytes32 value) internal {
        self.data[self.head] = value;
        self.head = (self.head + 1) % self.capacity;
        if (self.count < self.capacity) self.count++;
    }

    function popFront(RingBuffer storage self, uint256 n) internal {
        if (n > self.count) revert NotEnoughItems();
        self.count -= n;
    }

    function get(
        RingBuffer storage self,
        uint256 i
    ) internal view returns (bytes32) {
        if (i >= self.count) revert IndexOutOfRange();
        uint256 start = (self.head + self.capacity - self.count) %
            self.capacity;
        return self.data[(start + i) % self.capacity];
    }

    function isFull(RingBuffer storage self) internal view returns (bool) {
        return self.count == self.capacity;
    }
}

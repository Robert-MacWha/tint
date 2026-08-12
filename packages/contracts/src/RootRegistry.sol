// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @notice Tracks the monotonically-advancing sequence of Merkle roots produced by operations.
///
/// Each root is assigned a one-based index. An operation providing oldRoot must reference a
/// registered root; the new root gets index roots[oldRoot]+1 if that root exceeds the current tip.
contract RootRegistry {
    mapping(bytes32 root => uint128 index) public roots;
    uint128 public currentRootIndex;

    error InvalidOldRoot();

    constructor(bytes32 genesisRoot) {
        // starting root at index 1. Zero is a magic number for "unregistered root".
        roots[genesisRoot] = 1;
        currentRootIndex = 1;
    }

    /// @notice Reverts if oldRoot has no recorded index.
    function _validateOldRoot(bytes32 oldRoot) internal view {
        if (roots[oldRoot] == 0) revert InvalidOldRoot();
    }

    /// @notice Increments the root index if oldRoot is the current tip, and
    /// records newRoot at that index.
    function _updateRoot(bytes32 oldRoot, bytes32 newRoot) internal {
        if (oldRoot == newRoot) return;

        uint128 oldIdx = roots[oldRoot];
        if (oldIdx != currentRootIndex) return;

        uint128 newIdx = oldIdx + 1;
        roots[newRoot] = newIdx;
        currentRootIndex = newIdx;
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @notice Tracks the monotonically-advancing sequence of Merkle roots produced by operations.
///
/// Each root is assigned a one-based index. An operation providing oldRoot must reference a
/// registered root; the new root gets index roots[oldRoot]+1, but only if that index exceeds
/// the current tip — ensuring the sequence never rolls back.
contract RootRegistry {
    mapping(bytes32 root => uint128 index) public roots;
    uint128 public currentRootIndex;

    error InvalidOldRoot();

    constructor(bytes32 genesisRoot) {
        roots[genesisRoot] = 1; // genesis root at one-based index 1
        currentRootIndex = 1;
    }

    /// Reverts if oldRoot has no recorded index (was never registered).
    function _validateOldRoot(bytes32 oldRoot) internal view {
        if (roots[oldRoot] == 0) revert InvalidOldRoot();
    }

    /// Registers newRoot at roots[oldRoot]+1 only if that index strictly exceeds currentRootIndex.
    function _updateRoot(bytes32 oldRoot, bytes32 newRoot) internal {
        uint128 newIdx = roots[oldRoot] + 1;
        if (newIdx > currentRootIndex) {
            currentRootIndex = newIdx;
            roots[newRoot] = newIdx;
        }
    }
}

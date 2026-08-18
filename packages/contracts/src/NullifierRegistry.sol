// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract NullifierRegistry {
    mapping(bytes32 nullifierHash => bool spent) public isSpent;

    error NullifierAlreadySpent(bytes32 nullifier);

    /// @notice Reverts if the nullifier has already been spent.
    function _requireUnspent(bytes32 hash) internal view {
        if (hash == bytes32(0)) return;
        if (isSpent[hash]) revert NullifierAlreadySpent(hash);
    }

    /// @notice Marks the nullifier as spent. Reverts if the nullifier
    /// has already been spent.
    ///
    /// @dev Ignores nullifiers with the magic value of 0.
    function _spend(bytes32 hash) internal {
        if (hash == bytes32(0)) return;
        _requireUnspent(hash);
        isSpent[hash] = true;
    }
}

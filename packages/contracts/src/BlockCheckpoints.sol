// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @notice Tracks recently active blocks for use when syncing.
contract BlockCheckpoints {
    uint256 internal constant CHECKPOINT_LEVELS = 4;
    uint256 internal constant CHECKPOINT_SPACING = 100;

    /// @notice Recently active blocks.
    ///
    /// `checkpoints[i]` is set to the most recent block once it's at least
    /// `CHECKPOINT_SPACING ** i` blocks behind the current block.
    uint64[CHECKPOINT_LEVELS] public checkpoints;

    event Checkpoint(uint64[CHECKPOINT_LEVELS] previous);

    /// @notice Records this block as active, emitting the previous checkpoints on
    /// the first call in each block.
    function _checkpoint() internal {
        uint64[CHECKPOINT_LEVELS] memory previous = checkpoints;
        if (previous[0] == block.number) return;

        uint256 spacing = 1;
        for (uint256 i; i < CHECKPOINT_LEVELS; ++i) {
            // forge-lint: disable-next-line(unsafe-typecast)
            if (block.number - previous[i] >= spacing) checkpoints[i] = uint64(block.number);
            spacing *= CHECKPOINT_SPACING;
        }

        emit Checkpoint(previous);
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {LibSkewMmr} from "./LibSkewMmr.sol";

library LibSkewMmrWithHistory {
    using LibSkewMmr for LibSkewMmr.State;

    /// Maximum number of frontier commitments to store in history.
    ///
    /// @dev Bounds how stale a caller-supplied frontier may be. One append per deposit and
    ///      one per output commitment, so this is the proving window users race against.
    ///      Note `prewarm` writes one slot per entry, so raising it costs ~20k gas each at
    ///      deploy time.
    uint256 public constant HISTORY_SIZE = 256;

    struct State {
        LibSkewMmr.State mmr;

        /// @dev `history[count % HISTORY_SIZE]` commits to the frontier at `count`.
        bytes32[HISTORY_SIZE] history;
    }

    error BadFrontier();
    error UnknownFrontier();

    /// @dev Warms up storage with non-zero values to avoid cold SSTOREs.
    function prewarm(State storage self) internal {
        self.mmr.prewarm();
        for (uint256 i = 0; i < HISTORY_SIZE; ++i) {
            self.history[i] = bytes32(uint256(1));
        }
    }

    /// @notice Append an element to the MMR and store the resulting frontier to history.
    /// @dev See `LibSkewMmr.append` for more details.
    ///
    /// @return top The chain folding every root of the resulting frontier.
    function append(
        State storage self,
        bytes32 element,
        function(bytes32, bytes32, bytes32) internal view returns (bytes32) hash
    ) internal returns (bytes32 top) {
        top = self.mmr.append(element, hash);

        uint256 s = self.mmr.state;
        self.history[LibSkewMmr._count(s) % HISTORY_SIZE] = LibSkewMmr._link(top, bytes32(s));
    }

    /// @notice Reverts unless `frontier` and `histState` were the frontier and state this MMR
    ///         held at `_count(histState)`.
    function verifyFrontier(State storage self, uint256 histState, bytes32[] calldata frontier) internal view {
        if (frontier.length != LibSkewMmr._depth(histState)) revert BadFrontier();

        bytes32 chain = LibSkewMmr.IV;
        for (uint256 i = 0; i < frontier.length; ++i) {
            chain = LibSkewMmr._link(chain, frontier[i]);
        }

        if (self.history[LibSkewMmr._count(histState) % HISTORY_SIZE] != LibSkewMmr._link(chain, bytes32(histState))) {
            revert UnknownFrontier();
        }
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

library LibSkewMmr {
    /// Maximum number of trees in the frontier.
    uint256 public constant MAX_DEPTH = 26;

    /// @dev Initial value for the tree folding chain.
    bytes32 internal constant IV = bytes32(0);

    struct State {
        /// @dev The frontier: tree roots, smallest-ranked last. Entries above `depth` are undefined.
        bytes32[MAX_DEPTH] roots;

        /// @dev 0..25  - `rank`s
        ///      26     - `depth`
        ///      27..30 - `count`
        ///      31     - unused, set at construction so the slot is never zero
        uint256 state;

        /// @dev `chains[i]` folds `roots[0..i]`. Entries above `depth` are undefined.
        bytes32[MAX_DEPTH] chains;
    }

    error TooDeep();

    /// @dev Warms up storage with non-zero values to avoid cold SSTOREs.
    function prewarm(State storage self) internal {
        self.state = 1 << 248;
        for (uint256 i = 0; i < MAX_DEPTH; ++i) {
            self.roots[i] = bytes32(uint256(1));
            self.chains[i] = bytes32(uint256(1));
        }
    }

    /// @notice Append an element to the MMR.
    /// @return top The chain folding every root of the resulting frontier.
    ///
    /// @param element The new elelment to append to the MMR.
    /// @param hash Merges a new element with the two roots it subsumes into a new tree root.
    ///
    /// @dev Maximum of 1 hash per append. Follows skew-binary carry rules:
    ///      1. If the two smallest trees have the same rank, merge them into a tree of rank+1.
    ///      2. Otherwise, append the new element as a tree of rank 0.
    ///
    ///      Either branch writes `roots[tree]`, and `chains[tree - 1]` is stable across it.
    function append(
        State storage self,
        bytes32 element,
        function(bytes32, bytes32, bytes32) internal view returns (bytes32) hash
    ) internal returns (bytes32 top) {
        uint256 s = self.state;
        uint256 d = _depth(s);
        uint256 tree;
        uint256 rank;
        bytes32 root;

        if (d >= 2 && _rank(s, d - 1) == _rank(s, d - 2)) {
            root = hash(element, self.roots[d - 1], self.roots[d - 2]);
            self.roots[d - 2] = root;

            tree = d - 2;
            rank = _rank(s, d - 1) + 1;
            d -= 1;
            s = _setRank(s, d, 0);
        } else {
            if (d == MAX_DEPTH) revert TooDeep();

            root = element;
            self.roots[d] = root;

            tree = d;
            rank = 0;
            d += 1;
        }

        top = _link(tree >= 1 ? self.chains[tree - 1] : IV, root);
        self.chains[tree] = top;

        s = _setRank(s, tree, rank);
        s = _setDepth(s, d);
        self.state = _incrementCount(s);
    }

    function ranks(State storage self, uint256 tree) internal view returns (uint8) {
        return uint8(_rank(self.state, tree));
    }

    function depth(State storage self) internal view returns (uint256) {
        return _depth(self.state);
    }

    function count(State storage self) internal view returns (uint256) {
        return _count(self.state);
    }

    function _link(bytes32 prev, bytes32 root) internal pure returns (bytes32 chain) {
        assembly ("memory-safe") {
            mstore(0x00, prev)
            mstore(0x20, root)
            chain := keccak256(0x00, 0x40)
        }
    }

    function _rank(uint256 s, uint256 tree) internal pure returns (uint256) {
        return (s >> (8 * tree)) & 0xff;
    }

    function _setRank(uint256 s, uint256 tree, uint256 rank) internal pure returns (uint256) {
        uint256 offset = 8 * tree;
        return (s & ~(uint256(0xff) << offset)) | (rank << offset);
    }

    function _depth(uint256 s) internal pure returns (uint256) {
        return (s >> 208) & 0xff;
    }

    function _setDepth(uint256 s, uint256 d) internal pure returns (uint256) {
        return (s & ~(uint256(0xff) << 208)) | (d << 208);
    }

    function _count(uint256 s) internal pure returns (uint256) {
        return (s >> 216) & 0xffffffff;
    }

    function _incrementCount(uint256 s) internal pure returns (uint256) {
        return s + (uint256(1) << 216);
    }
}

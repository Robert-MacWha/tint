// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {ProofLib} from "./lib/ProofLib.sol";
import {N_PUB, N_COMPRESSED_PUB} from "./lib/Constants.sol";

/// @notice Exposes `ProofLib.toCompressedSignals` externally for testing.
contract HybridCompression {
    function toCompressedSignals(uint256[N_PUB] calldata pub, uint256 beta)
        external
        pure
        returns (uint256[N_COMPRESSED_PUB] memory)
    {
        return ProofLib.toCompressedSignals(pub, beta);
    }
}

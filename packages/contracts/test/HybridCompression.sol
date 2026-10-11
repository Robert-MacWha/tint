// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {LibHybridCompression} from "../src/lib/LibHybridCompression.sol";
import {N_PUB, N_COMPRESSED_PUB, BN254_FR_MODULUS} from "../src/lib/Constants.sol";

/// @notice Exposes `LibHybridCompression.hybridCompression` externally for testing.
contract HybridCompression {
    function toCompressedSignals(uint256[N_PUB] calldata pub, uint256 beta)
        external
        pure
        returns (uint256[N_COMPRESSED_PUB] memory)
    {
        uint256[] memory stmt = new uint256[](N_PUB);
        for (uint256 i = 0; i < N_PUB; i++) {
            stmt[i] = pub[i];
        }
        (uint256 alpha, uint256 gamma) = LibHybridCompression.hybridCompression(beta, stmt, BN254_FR_MODULUS);
        return [alpha, beta, gamma];
    }
}

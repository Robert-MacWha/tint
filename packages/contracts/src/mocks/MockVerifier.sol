// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IVerifier} from "../interfaces/IVerifier.sol";
import {N_PUB} from "../lib/Constants.sol";

/// @notice Always-accepting verifier for local development and testing only.
/// @dev Never use in production — this performs no actual proof verification.
contract MockVerifier is IVerifier {
    function verifyProof(uint256[2] calldata, uint256[2][2] calldata, uint256[2] calldata, uint256[N_PUB] calldata)
        external
        pure
        returns (bool)
    {
        return true;
    }
}

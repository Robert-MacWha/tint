// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IVerifier} from "../interfaces/IVerifier.sol";
import {N_PUB} from "../lib/Constants.sol";

/// @notice Always-accepting verifier for local development and testing only.
/// @dev Never use in production — this performs no actual proof verification.
contract MockVerifier is IVerifier {
    function verifyProof(
        uint[2] calldata,
        uint[2][2] calldata,
        uint[2] calldata,
        uint[N_PUB] calldata
    ) external pure returns (bool) {
        return true;
    }
}

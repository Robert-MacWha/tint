// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IVerifier} from "../interfaces/IVerifier.sol";
import {N_PUB} from "../lib/Constants.sol";

/// @notice Always-accepting verifier for local development and testing only.
contract MockVerifier is IVerifier {
    function verify(uint256[8] calldata, uint256[N_PUB] calldata) external pure {
        // Always accept
    }
}

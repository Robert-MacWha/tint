// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ProofLib} from "../src/lib/ProofLib.sol";

/// @notice Exposes `ProofLib.toPublicSignals` externally for testing.
contract PublicSignals {
    using ProofLib for IPrivacyPool.Operation;

    function toPublicSignals(IPrivacyPool.Operation calldata op) external pure returns (uint256[] memory) {
        return op.toPublicSignals();
    }
}

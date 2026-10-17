// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPrivacyPool} from "../../src/interfaces/IPrivacyPool.sol";
import {IPoseidon2T2} from "../../src/interfaces/IPoseidon2T2.sol";
import {ProofLib} from "../../src/lib/ProofLib.sol";

/// @notice Exposes `ProofLib.signingHash` externally for testing.
contract SigningHash {
    using ProofLib for IPrivacyPool.Operation;

    IPoseidon2T2 public immutable POSEIDON;

    constructor(IPoseidon2T2 poseidon) {
        POSEIDON = poseidon;
    }

    function signingHash(IPrivacyPool.Operation calldata op) external view returns (uint256) {
        return op.signingHash(POSEIDON);
    }
}

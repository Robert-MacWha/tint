// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {N_INPUTS, N_OUTPUTS} from "../lib/Constants.sol";
import {ProofLib} from "../lib/ProofLib.sol";

interface IPrivacyPool {
    struct Operation {
        /// @dev Read the typed fields through the `ProofLib` accessors.
        bytes32[] pub;
        Context context;
        ProofLib.Proof proof;

        /// @dev Hybrid-compression challenge for `proof` (see
        /// `ProofLib.toCompressedSignals`).
        uint256 beta;
    }

    struct Context {
        bytes[N_INPUTS] policyInputs;
        bytes[N_OUTPUTS] ciphertexts;
    }

    function deposit(address asset, uint128 amount, bytes32 partialCommitment, bytes calldata encryptedNote) external;

    function operate(Operation calldata operation) external;
    // function preVerify(bytes32 slot, Operation calldata operation) external;
    // function executePreVerified(bytes32 slot, Operation calldata operation) external;
}

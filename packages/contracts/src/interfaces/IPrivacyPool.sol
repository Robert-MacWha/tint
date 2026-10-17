// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {N_INPUTS, N_OUTPUTS} from "../lib/Constants.sol";
import {ProofLib} from "../lib/ProofLib.sol";

interface IPrivacyPool {
    struct Operation {
        /// Read the typed fields through the `ProofLib` accessors.
        bytes32[] pub;
        Context context;
        ProofLib.Proof proof;

        /// Hybrid-compression challenge for `proof` (see
        /// `ProofLib.toCompressedSignals`).
        uint256 beta;
    }

    /// Context for the operation not included in the proof. The contents
    /// of this struct are hashed and bound to the proof so that they cannot
    /// be changed without invalidating the proof.
    struct Context {
        /// If non-zero, verifies that msg.sender is equal to this address.
        address caller;
        /// Opaque binding usable by `caller` to bind arbitrary data.
        bytes32 binding;
        /// Opaque inputs for policies to bind their calldata.
        bytes[N_INPUTS] policyInputs;
        /// The encrypted ciphertext outputs for the operation.
        bytes[N_OUTPUTS] ciphertexts;
    }

    function deposit(address asset, uint128 amount, bytes32 partialCommitment, bytes calldata encryptedNote) external;

    function operate(Operation calldata operation) external;
    // function preVerify(bytes32 slot, Operation calldata operation) external;
    // function executePreVerified(bytes32 slot, Operation calldata operation) external;
}

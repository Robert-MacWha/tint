// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {N_INPUTS, N_OUTPUTS} from "../lib/Constants.sol";
import {ProofLib} from "../lib/ProofLib.sol";

interface IPrivacyPool {
    struct Operation {
        bytes32 oldRoot;
        uint64 oldRootLength;
        bytes32 startAggregationHash;
        bytes32 newRoot;
        uint128 leavesAggregationIndex;
        bytes32[N_INPUTS] nullifiers;
        bytes32[N_OUTPUTS] commitmentsOut;
        uint128[N_OUTPUTS] unshieldAmounts;
        address[N_OUTPUTS] unshieldAssets;
        // bound params
        address[N_OUTPUTS] unshieldRecipients;
        address[N_OUTPUTS] spendabilityAddresses;
        bytes32[N_OUTPUTS] spendabilityData;
        bytes[N_OUTPUTS] encryptedNotes;
        ProofLib.Proof proof;
    }

    function deposit(
        address asset,
        uint128 amount,
        bytes32 commitment,
        bytes calldata encryptedNote
    ) external;

    function operate(Operation calldata operation) external;
}

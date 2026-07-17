// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {N_INPUTS, N_OUTPUTS, N_WITHDRAWALS} from "../lib/Constants.sol";
import {ProofLib} from "../lib/ProofLib.sol";

interface IPrivacyPool {
    struct Operation {
        bytes32 oldRoot;
        uint128 startAggregationIndex;
        bytes32 newRoot;
        uint128 endAggregationIndex;
        bytes32[N_INPUTS] nullifiers;
        address[N_INPUTS] spendabilityAddresses;
        bytes32[N_OUTPUTS] commitmentsOut;
        uint128[N_WITHDRAWALS] unshieldAmounts;
        address[N_WITHDRAWALS] unshieldAssets;
        Context context;
        ProofLib.Proof proof;
    }

    struct Context {
        bytes[N_INPUTS] spendabilityInputs;
        bytes[N_OUTPUTS] ciphertexts;
        address[N_WITHDRAWALS] unshieldRecipients;
    }

    function deposit(
        address asset,
        uint128 amount,
        bytes32 commitment,
        bytes calldata encryptedNote
    ) external;

    function operate(Operation calldata operation) external;
    function preVerify(bytes32 slot, Operation calldata operation) external;
    function executePreVerified(
        bytes32 slot,
        Operation calldata operation
    ) external;
}

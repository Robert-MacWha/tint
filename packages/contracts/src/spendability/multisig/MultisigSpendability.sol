// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ISpendability} from "../../interfaces/ISpendability.sol";
import {IPrivacyPool} from "../../interfaces/IPrivacyPool.sol";
import {ProofLib} from "../../lib/ProofLib.sol";
import {N_INPUTS} from "../../lib/Constants.sol";
import {Verifier} from "./MultisigSpendabilityVerifier.sol";

/// @notice Multisig spendability rule that verifies M-of-N signatures.
contract MultisigSpendability is ISpendability {
    Verifier public immutable VERIFIER;

    error InvalidProof();
    error NoInputsForThisSpendability();

    constructor(Verifier verifier) {
        VERIFIER = verifier;
    }

    function requireSpendable(
        IPrivacyPool.Operation calldata operation
    ) external view {
        for (uint256 i = 0; i < N_INPUTS; i++) {
            if (operation.spendabilityAddresses[i] != address(this)) continue;
            bytes calldata proof = operation.context.spendabilityInputs[i];
            uint256[2] memory pubSignals = [
                uint256(uint160(address(this))),
                uint256(operation.operationHash)
            ];

            VERIFIER.verifyProof(proof, pubSignals);
            return;
        }

        revert NoInputsForThisSpendability();
    }
}

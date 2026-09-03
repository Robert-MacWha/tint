// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {ISpendability} from "../../interfaces/ISpendability.sol";
import {IPrivacyPool} from "../../interfaces/IPrivacyPool.sol";
import {Verifier} from "../../codegen/PasswordVerifier.sol";
import {ProofLib} from "../../lib/ProofLib.sol";
import {N_INPUTS} from "../../lib/Constants.sol";

/// @notice Password spendability rule that verifies knowledge of a password
/// when spending a note.
contract PasswordSpendability is ISpendability {
    Verifier public immutable VERIFIER;

    error InvalidProof();
    error NoInputsForThisSpendability();

    constructor(Verifier verifier) {
        VERIFIER = verifier;
    }

    function requireSpendable(IPrivacyPool.Operation calldata operation) external view {
        for (uint256 i = 0; i < N_INPUTS; i++) {
            if (operation.spendabilityAddresses[i] != address(this)) continue;

            ProofLib.Proof memory proof = abi.decode(operation.context.spendabilityInputs[i], (ProofLib.Proof));
            uint256[2] memory pubSignals = [uint256(uint160(address(this))), uint256(operation.operationHash)];

            VERIFIER.verifyProof(proof.proof, pubSignals);

            return;
        }

        revert NoInputsForThisSpendability();
    }
}

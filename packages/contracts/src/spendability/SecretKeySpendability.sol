// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ISpendability} from "../interfaces/ISpendability.sol";
import {IPrivacyPool} from "../interfaces/IPrivacyPool.sol";
import {ProofLib} from "../lib/ProofLib.sol";
import {N_INPUTS} from "../lib/Constants.sol";
import {
    SecretKeySpendabilityVerifier
} from "./SecretKeySpendabilityVerifier.sol";

/// @notice Secret key spendability rule that verifies knowledge of a secret
/// key when spending a note.
contract SecretKeySpendability is ISpendability {
    SecretKeySpendabilityVerifier public immutable VERIFIER;

    constructor(SecretKeySpendabilityVerifier verifier) {
        VERIFIER = verifier;
    }

    function isSpendable(
        IPrivacyPool.Operation calldata operation
    ) external view returns (bool) {
        for (uint256 i = 0; i < N_INPUTS; i++) {
            if (operation.spendabilityAddresses[i] != address(this)) continue;

            ProofLib.Proof memory proof = abi.decode(
                operation.context.spendabilityInputs[i],
                (ProofLib.Proof)
            );
            uint256[2] memory pubSignals = [
                uint256(uint160(address(this))),
                uint256(operation.operationHash)
            ];

            return
                VERIFIER.verifyProof(proof.pA, proof.pB, proof.pC, pubSignals);
        }

        return false;
    }
}

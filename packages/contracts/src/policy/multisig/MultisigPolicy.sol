// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPolicy} from "../../interfaces/IPolicy.sol";
import {IPrivacyPool} from "../../interfaces/IPrivacyPool.sol";
import {IPoseidon2T2} from "../../interfaces/IPoseidon2T2.sol";
import {ProofLib} from "../../lib/ProofLib.sol";
import {N_INPUTS} from "../../lib/Constants.sol";
import {Verifier} from "../../codegen/MultisigVerifier.sol";

/// @notice Multisig policy rule that verifies M-of-N signatures.
contract MultisigPolicy is IPolicy {
    using ProofLib for IPrivacyPool.Operation;
    Verifier public immutable VERIFIER;
    IPoseidon2T2 public immutable POSEIDON;

    error InvalidProof();
    error NoInputsForThisPolicy();

    constructor(Verifier verifier, IPoseidon2T2 poseidon) {
        VERIFIER = verifier;
        POSEIDON = poseidon;
    }

    function requireAuthorized(IPrivacyPool.Operation calldata operation) external view {
        for (uint256 i = 0; i < N_INPUTS; i++) {
            if (operation.policyAddress(i) != address(this)) continue;
            bytes calldata proof = operation.context.policyInputs[i];
            uint256[2] memory pubSignals = [uint256(uint160(address(this))), operation.signingHash(POSEIDON)];

            VERIFIER.verifyProof(proof, pubSignals);
            return;
        }

        revert NoInputsForThisPolicy();
    }
}

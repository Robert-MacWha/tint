// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPolicy} from "../../interfaces/IPolicy.sol";
import {IPrivacyPool} from "../../interfaces/IPrivacyPool.sol";
import {IPoseidon2T2} from "../../interfaces/IPoseidon2T2.sol";
import {Verifier} from "../../codegen/PasswordVerifier.sol";
import {ProofLib} from "../../lib/ProofLib.sol";
import {N_INPUTS} from "../../lib/Constants.sol";

/// @notice Password policy rule that verifies knowledge of a password
/// when spending a note.
contract PasswordPolicy is IPolicy {
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

            ProofLib.Proof memory proof = abi.decode(operation.context.policyInputs[i], (ProofLib.Proof));
            uint256[2] memory pubSignals = [uint256(uint160(address(this))), operation.signingHash(POSEIDON)];

            VERIFIER.verifyProof(proof.proof, pubSignals);

            return;
        }

        revert NoInputsForThisPolicy();
    }
}

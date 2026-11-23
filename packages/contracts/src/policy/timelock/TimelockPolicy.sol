// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPolicy} from "../../interfaces/IPolicy.sol";
import {IPrivacyPool} from "../../interfaces/IPrivacyPool.sol";
import {IPoseidon2T2} from "../../interfaces/IPoseidon2T2.sol";
import {Verifier} from "../../codegen/TimelockVerifier.sol";
import {ProofLib} from "../../lib/ProofLib.sol";
import {N_INPUTS} from "../../lib/Constants.sol";

/// @notice Timelock policy rule that only lets a note be spent once its
/// committed unlock time has passed.
/// @dev The policy input is `abi.encode(proof, bound)`. The proof shows the
/// unlock time is at most `bound`, so the prover can reveal a coarse `bound`
/// rather than the exact unlock time.
contract TimelockPolicy is IPolicy {
    using ProofLib for IPrivacyPool.Operation;
    Verifier public immutable VERIFIER;
    IPoseidon2T2 public immutable POSEIDON;

    error Locked();
    error NoInputsForThisPolicy();

    constructor(Verifier verifier, IPoseidon2T2 poseidon) {
        VERIFIER = verifier;
        POSEIDON = poseidon;
    }

    function requireAuthorized(IPrivacyPool.Operation calldata operation) external view {
        for (uint256 i = 0; i < N_INPUTS; i++) {
            if (operation.policyAddress(i) != address(this)) continue;

            (ProofLib.Proof memory proof, uint256 bound) =
                abi.decode(operation.context.policyInputs[i], (ProofLib.Proof, uint256));
            uint256[3] memory pubSignals = [uint256(uint160(address(this))), operation.signingHash(POSEIDON), bound];

            // Verify before the bound check, so placeholder inputs spend a valid input's gas.
            VERIFIER.verifyProof(proof.proof, pubSignals);
            if (bound > block.timestamp) revert Locked();

            return;
        }

        revert NoInputsForThisPolicy();
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPolicy} from "../../interfaces/IPolicy.sol";
import {IPrivacyPool} from "../../interfaces/IPrivacyPool.sol";
import {IPoseidon2T2} from "../../interfaces/IPoseidon2T2.sol";
import {Verifier} from "../../codegen/IntentVerifier.sol";
import {ProofLib} from "../../lib/ProofLib.sol";
import {N_INPUTS} from "../../lib/Constants.sol";

/// @notice Intent policy rule: a note is spendable by an operation that creates
/// the note its owner wants before a deadline, or by its owner's cancel key.
/// @dev Each input bound to this policy carries its own proof, so one operation
/// can spend several intents. See `docs/intents.md`.
contract IntentPolicy is IPolicy {
    using ProofLib for IPrivacyPool.Operation;
    Verifier public immutable VERIFIER;
    IPoseidon2T2 public immutable POSEIDON;

    error Expired();
    error NoInputsForThisPolicy();

    constructor(Verifier verifier, IPoseidon2T2 poseidon) {
        VERIFIER = verifier;
        POSEIDON = poseidon;
    }

    /// @dev Each bound input's policy input is `abi.encode(ProofLib.Proof, uint64 deadline)`.
    function requireAuthorized(IPrivacyPool.Operation calldata operation) external view {
        uint256 signingHash = operation.signingHash(POSEIDON);
        bool found = false;

        for (uint256 i = 0; i < N_INPUTS; i++) {
            if (operation.policyAddress(i) != address(this)) continue;

            (ProofLib.Proof memory proof, uint64 deadline) =
                abi.decode(operation.context.policyInputs[i], (ProofLib.Proof, uint64));
            if (block.timestamp > deadline) revert Expired();

            uint256[4] memory pubSignals = [uint256(uint160(address(this))), signingHash, i, deadline];
            VERIFIER.verifyProof(proof.proof, pubSignals);
            found = true;
        }

        if (!found) revert NoInputsForThisPolicy();
    }
}

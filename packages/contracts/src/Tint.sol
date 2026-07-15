// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {
    SafeERC20,
    IERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {IVerifier} from "./interfaces/IVerifier.sol";
import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {N_INPUTS, N_OUTPUTS, N_PUB, GENESIS_ROOT} from "./lib/Constants.sol";
import {ProofLib} from "./lib/ProofLib.sol";
import {AggregationRing} from "./AggregationRing.sol";
import {RootRegistry} from "./RootRegistry.sol";

/// @notice Privacy-preserving token pool using zk-snarks and a merkle tree accumulator.
contract Tint is IPrivacyPool, AggregationRing, RootRegistry {
    using SafeERC20 for IERC20;

    IVerifier public immutable VERIFIER;

    mapping(bytes32 nullifierHash => bool spent) public nullifierHashes;

    event Deposited(bytes32 commitment, bytes encryptedNote);
    event Committed(bytes32 commitment, bytes encryptedNote);
    event Nullified(bytes32 nullifier);
    event Withdrawn(
        address indexed asset,
        uint128 amount,
        address indexed recipient
    );

    error ZeroAmount();
    error ZeroCommitment();
    error InvalidProof();
    error NullifierAlreadySpent(bytes32 nullifier);
    error UnshieldRecipientZero(uint256 index);

    constructor(address _verifier) RootRegistry(GENESIS_ROOT) {
        VERIFIER = IVerifier(_verifier);
    }

    /// @notice Deposits an asset into the pool and queues the commitment for aggregation.
    ///
    /// @param asset The ERC20 token contract address.
    /// @param amount The amount to deposit in.
    /// @param partialCommitment The partial commitment for the private output note.
    ///
    /// @dev The caller must have approved this contract to spend at least `amount` of `asset`.
    function deposit(
        address asset,
        uint128 amount,
        bytes32 partialCommitment,
        bytes calldata encryptedNote
    ) external {
        if (amount == 0) revert ZeroAmount();
        if (partialCommitment == 0) revert ZeroCommitment();

        bytes32 commitment = ProofLib.toCommitment(
            asset,
            amount,
            partialCommitment
        );
        _commit(commitment);
        IERC20(asset).safeTransferFrom(msg.sender, address(this), amount);
        emit Deposited(commitment, encryptedNote);
    }

    function operate(IPrivacyPool.Operation calldata operation) public {
        verifyOperation(operation);
        _executeOperation(operation);
    }

    /// @notice Computes the Groth16 public-signal vector `op` must satisfy.
    /// Exposed so a client can cross-check its locally-computed proof inputs
    /// against the contract's, rather than debugging an opaque
    /// `InvalidProof` revert.
    function computePublicSignals(
        IPrivacyPool.Operation calldata op
    ) public view returns (uint256[N_PUB] memory) {
        bytes32 startAggregationHash = _getHash(op.startAggregationIndex);
        bytes32 endAggregationHash = _getHash(op.endAggregationIndex);

        bytes32 boundParamsHash = ProofLib.toBoundParamsHash(
            op.unshieldRecipients
        );

        return
            ProofLib.toPublicSignals(
                op.oldRoot,
                op.startAggregationIndex,
                startAggregationHash,
                boundParamsHash,
                op.newRoot,
                endAggregationHash,
                op.nullifiers,
                op.commitmentsOut,
                op.unshieldAmounts,
                op.unshieldAssets
            );
    }

    /// @notice Verifies that the provided operation is valid or reverts if not.
    ///
    /// TODO: Ensure there's enough space for new commitments so `_commit` doesn't revert.
    function verifyOperation(IPrivacyPool.Operation calldata op) public view {
        _validateOldRoot(op.oldRoot);

        uint256[N_PUB] memory pubSignals = computePublicSignals(op);
        if (
            !VERIFIER.verifyProof(
                op.proof.pA,
                op.proof.pB,
                op.proof.pC,
                pubSignals
            )
        ) {
            revert InvalidProof();
        }

        for (uint256 i; i < N_INPUTS; ++i) {
            bytes32 hash = op.nullifiers[i];
            if (hash == 0) continue; // dummy input slot
            if (nullifierHashes[hash]) revert NullifierAlreadySpent(hash);
        }

        for (uint256 i; i < N_OUTPUTS; ++i) {
            if (op.unshieldAmounts[i] == 0) continue;
            if (op.unshieldRecipients[i] == address(0))
                revert UnshieldRecipientZero(i);
        }
    }

    /// @notice Executes the state changes specified by the operation.
    /// @dev Assumes the operation has already been verified.
    function _executeOperation(IPrivacyPool.Operation calldata op) private {
        // Nullify the input notes
        for (uint256 i; i < N_INPUTS; ++i) {
            bytes32 hash = op.nullifiers[i];
            if (hash == 0) continue; // dummy input slot
            nullifierHashes[hash] = true;
            emit Nullified(hash);
        }

        // Stage any output commitments
        for (uint256 i; i < N_OUTPUTS; ++i) {
            bytes32 commitment = op.commitmentsOut[i];
            if (commitment == 0) continue;
            _commit(commitment);
            emit Committed(commitment, op.encryptedNotes[i]);
        }

        _advanceConsumed(op.endAggregationIndex);
        _updateRoot(op.oldRoot, op.newRoot);

        // Execute any unshielding transfers
        for (uint256 i; i < N_OUTPUTS; ++i) {
            address asset = op.unshieldAssets[i];
            uint128 amount = op.unshieldAmounts[i];
            address recipient = op.unshieldRecipients[i];
            if (amount == 0) continue;
            IERC20(asset).safeTransfer(recipient, amount);
            emit Withdrawn(asset, amount, recipient);
        }
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {
    SafeERC20,
    IERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {IVerifier} from "./interfaces/IVerifier.sol";
import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {N_INPUTS, N_OUTPUTS, N_PUB} from "./lib/Constants.sol";
import {ProofLib} from "./lib/ProofLib.sol";
import {AggregationRing} from "./AggregationRing.sol";
import {RootRegistry} from "./RootRegistry.sol";

/// @notice Privacy-preserving token pool using zk-snarks and a merkle tree accumulator.
contract Tint is IPrivacyPool, AggregationRing, RootRegistry {
    using SafeERC20 for IERC20;

    IVerifier public immutable VERIFIER;

    mapping(bytes32 nullifierHash => bool spent) public nullifierHashes;

    event Deposited(
        address indexed asset,
        uint128 amount,
        bytes32 partialCommitment,
    );
    event Nullified(bytes32 indexed nullifier);
    event Committed(
        bytes32 indexed commitment,
        address indexed spendabilityAddress,
        bytes spendabilityData
    );
    event Withdrawn(
        address indexed asset,
        uint128 amount,
        address indexed recipient
    );

    error ZeroAmount();
    error InvalidProof();
    error NullifierAlreadySpent(bytes32 nullifier);
    error UnshieldRecipientZero(uint256 index);

    constructor(address _verifier) {
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
    ) external {
        if (amount == 0) revert ZeroAmount();

        bytes32 commitment = ProofLib.toCommitment(asset, amount, partialCommitment);
        _commit(commitment);
        IERC20(asset).safeTransferFrom(msg.sender, address(this), amount);
        emit Deposited(
            asset,
            amount,
            partialCommitment
        );
    }

    /// @notice Pre-validates an operation, writing a flag to tstore to indicate that the operation is valid.
    ///
    /// @dev allows validation to be skipped in the `operate` function if the operation has already been validated.
    function preValidateOperation(IPrivacyPool.Operation calldata op) public {
        verifyOperation(op);
        // TODO: Store validation result in tstore in a way that's associated with the caller ERC-7562-style
        // so we can use this with paymasters.
    }

    function operate(IPrivacyPool.Operation calldata operation) public {
        verifyOperation(operation);
        _executeOperation(operation);
    }

    /// @notice Verifies that the provided operation is valid or reverts if not.
    ///
    /// TODO: Ensure there's enough space for new commitments so `_commit` doesn't revert.
    function verifyOperation(IPrivacyPool.Operation calldata op) public view {
        _validateOldRoot(op.oldRoot);

        bytes32 leavesAggregationHash = _getHash(op.leavesAggregationIndex);

        bytes32 boundParamsHash = ProofLib.toBoundParamsHash(
            op.spendabilityAddresses,
            op.spendabilityData,
            op.unshieldRecipients
        );

        uint256[N_PUB] memory pubSignals = ProofLib.toPublicSignals(
            op.oldRoot,
            op.newRoot,
            leavesAggregationHash,
            op.nullifiers,
            op.commitmentsOut,
            op.unshieldAmounts,
            op.unshieldAssets,
            boundParamsHash
        );
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
            emit Committed(
                commitment,
                op.spendabilityAddresses[i],
                op.spendabilityData[i]
            );
        }

        _advanceConsumed(op.leavesAggregationIndex);
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

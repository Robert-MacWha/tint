// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {
    SafeERC20,
    IERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {PoseidonT3} from "poseidon-solidity/PoseidonT3.sol";

import {IVerifier} from "./interfaces/IVerifier.sol";
import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {
    N_INPUTS,
    N_OUTPUTS,
    N_PUB,
    AGGREGATION_RING_SIZE
} from "./lib/Constants.sol";
import {ProofLib} from "./lib/ProofLib.sol";

/// @notice Privacy-preserving token pool using zk-snarks and a merkle tree accumulator.
contract Tint is IPrivacyPool {
    using SafeERC20 for IERC20;

    IVerifier public immutable VERIFIER;

    uint128 public totalStaged; // total commitments ever staged
    uint128 public totalConsumed; // total commitments consumed by batches (packed with totalStaged)
    bytes32[AGGREGATION_RING_SIZE] public aggregationHashRing; // ring of recent aggregation hashes
    mapping(bytes32 nullifierHash => bool spent) public nullifierHashes;
    mapping(bytes32 root => uint256 index) public roots; // root -> one-based index (0 = invalid)
    uint256 public currentRootIndex; // one-based index of the latest root

    event Deposited(address indexed asset, uint128 amount, bytes32 commitment);
    event Nullified(bytes32 indexed nullifier);
    event Committed(bytes32 indexed commitment);
    event Withdrawn(
        address indexed asset,
        uint128 amount,
        address indexed recipient
    );

    error StagingFull();
    error InvalidProof();
    error NullifierAlreadySpent(bytes32 nullifier);
    error UnshieldRecipientZero(uint256 index);
    error InvalidOldRoot();
    error InvalidAggregationHash();

    constructor(address _verifier) {
        VERIFIER = IVerifier(_verifier);
        roots[bytes32(0)] = 1; // Genesis root at index 0
        currentRootIndex = 1; // Genesis root index
    }

    /// @notice Deposits an asset into the pool and queues the commitment for aggregation.
    ///
    /// @param asset The ERC20 token contract address.
    /// @param amount The amount to deposit in.
    /// @param commitment The commitment representing the private output note.
    ///
    /// @dev The caller must have approved this contract to spend at least `amount` of `asset`.
    function deposit(
        address asset,
        uint128 amount,
        bytes32 commitment
    ) external {
        _commit(commitment);
        IERC20(asset).safeTransferFrom(msg.sender, address(this), amount);
        emit Deposited(asset, amount, commitment);
    }

    function operate(IPrivacyPool.Operation[] calldata operations) external {
        for (uint256 i; i < operations.length; ++i) {
            IPrivacyPool.Operation calldata op = operations[i];
            _verifyOperation(op);
            _executeOperation(op);
        }
    }

    /// @notice Verifies that the provided operation is valid or reverts if not.
    function _verifyOperation(IPrivacyPool.Operation calldata op) private view {
        /// Check public inputs against current state
        if (roots[op.oldRoot] == 0) revert InvalidOldRoot();

        uint256 idx = op.leavesAggregationIndex;
        if (idx >= totalStaged || idx < totalConsumed)
            revert InvalidAggregationHash();
        bytes32 leavesAggregationHash = aggregationHashRing[
            idx % AGGREGATION_RING_SIZE
        ];

        /// Compute the public input array for the zk proof.
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

        /// Verify that the provided zk proof is valid for the operation's public inputs.
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

        /// Verify that none of the nullifiers have been previously spent.
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

        // Add any new commitments to the staging area
        for (uint256 i; i < N_OUTPUTS; ++i) {
            bytes32 commitment = op.commitmentsOut[i];

            if (commitment == 0) continue;
            _commit(commitment);
            emit Committed(commitment);
        }

        // Advance consumed pointer; ring slots recycle naturally via totalStaged % N
        uint256 idx = op.leavesAggregationIndex;
        if (idx >= totalConsumed) {
            totalConsumed = uint128(idx + 1);
        }

        // Update the merkle tree accumulator with the new root
        uint256 newIdx = roots[op.oldRoot] + 1;
        if (newIdx > currentRootIndex) {
            currentRootIndex = newIdx;
            roots[op.newRoot] = newIdx;
        }

        // Execute any unshielding transfers
        for (uint256 i; i < N_OUTPUTS; ++i) {
            address asset = op.unshieldAssets[i];
            uint128 amount = op.unshieldAmounts[i];
            address recipient = op.unshieldRecipients[i];
            if (amount == 0) continue;
            if (recipient == address(0)) revert UnshieldRecipientZero(i);

            IERC20(asset).safeTransfer(recipient, amount);
            emit Withdrawn(asset, amount, recipient);
        }
    }

    function _commit(bytes32 commitment) private {
        if (totalStaged - totalConsumed >= AGGREGATION_RING_SIZE)
            revert StagingFull();

        bytes32 prevHash = totalStaged > 0
            ? aggregationHashRing[(totalStaged - 1) % AGGREGATION_RING_SIZE]
            : bytes32(0);

        aggregationHashRing[totalStaged % AGGREGATION_RING_SIZE] = bytes32(
            PoseidonT3.hash([uint256(prevHash), uint256(commitment)])
        );

        totalStaged++;
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {SafeERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {IVerifier} from "./interfaces/IVerifier.sol";
import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {ISpendability} from "./interfaces/ISpendability.sol";
import {N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, N_PUB, AGGREGATION_RING_SIZE} from "./lib/Constants.sol";
import {ProofLib} from "./lib/ProofLib.sol";
import {LibAggregationRing} from "./lib/LibAggregationRing.sol";
import {LibPoseidon2T2_BN254} from "./lib/LibPoseidon2T2_BN254.sol";
import {NullifierRegistry} from "./NullifierRegistry.sol";

/// @notice Privacy-preserving token pool using zk-snarks and a merkle tree accumulator.
contract Tint is IPrivacyPool, NullifierRegistry {
    using SafeERC20 for IERC20;
    using LibAggregationRing for LibAggregationRing.AggregationRing;

    IVerifier public immutable VERIFIER;

    LibAggregationRing.AggregationRing internal ring;

    event Deposited(bytes32 commitment, bytes encryptedNote);
    event Committed(bytes32 commitment, bytes encryptedNote);
    event Nullified(bytes32 nullifier);
    event Withdrawn(address indexed asset, uint128 amount, address indexed recipient);
    event AggregationAdvanced(uint128 index, bytes32 root);

    error InvalidProof();

    constructor(address _verifier) {
        VERIFIER = IVerifier(_verifier);
        ring.init(AGGREGATION_RING_SIZE);
    }

    // -------------------- EXTERNAL STATE-CHANGING --------------------

    /// @notice Deposits an asset into the pool and queues the commitment for aggregation.
    ///
    /// @param asset The ERC20 token contract address.
    /// @param amount The amount to deposit in.
    /// @param partialCommitment The partial commitment for the private output note.
    ///
    /// @dev The caller must have approved this contract to spend at least `amount` of `asset`.
    function deposit(address asset, uint128 amount, bytes32 partialCommitment, bytes calldata encryptedNote) external {
        bytes32 commitment = ProofLib.toCommitment(asset, amount, partialCommitment);
        ring.stage(_hash, commitment);
        IERC20(asset).safeTransferFrom(msg.sender, address(this), amount);
        emit Deposited(commitment, encryptedNote);
    }

    /// @notice Executes an operation against tint.
    function operate(IPrivacyPool.Operation calldata operation) public {
        verifyOperation(operation);
        _executeOperation(operation);
    }

    /// @notice Pre-verifies an operation and stores its validity for later execution.
    ///
    /// @dev Pre-verified operations can be later executed with `executePreVerified`
    /// without re-verification.
    function preVerify(bytes32 slot, IPrivacyPool.Operation calldata operation) public {
        verifyOperation(operation);
        bytes32 operationHash = ProofLib.toOperationHash(operation);
        assembly {
            tstore(slot, operationHash)
        }
    }

    /// @notice Executes a pre-verified operation.
    ///
    /// @dev Assuming the operation has been pre-verified and that no erc20
    /// transfers revert, this function is guaranteed to not revert.
    function executePreVerified(bytes32 slot, IPrivacyPool.Operation calldata operation) public {
        bytes32 operationHash = ProofLib.toOperationHash(operation);
        bytes32 storedHash;

        // Check that the operation hash matches the stored pre-verified hash
        assembly {
            storedHash := tload(slot)
        }
        if (storedHash != operationHash) revert InvalidProof();

        // Clear the stored hash to prevent replay attacks
        assembly {
            tstore(slot, 0)
        }

        _executeOperation(operation);
    }

    // -------------------- EXTERNAL VIEW --------------------

    /// @notice Returns the aggregation index the pool has most recently advanced to.
    function latestRootIndex() external view returns (uint128) {
        return ring.latestRootIndex();
    }

    /// @notice Returns the root recorded at a given aggregation index.
    function getRoot(uint128 index) external view returns (bytes32) {
        return ring.getRoot(index);
    }

    /// @notice Returns the total number of commitments ever staged.
    function head() external view returns (uint128) {
        return ring.buffer.head;
    }

    /// @notice Returns the hash after `index` values have been staged.
    function getHash(uint128 index) external view returns (bytes32) {
        return ring.getHash(index);
    }

    /// @notice Returns the number of free slots in the aggregation ring.
    function space() external view returns (uint128) {
        return ring.space();
    }

    /// @notice Computes the Groth16 public-signal vector `op` must satisfy.
    /// Exposed so a client can cross-check its locally-computed proof inputs
    /// against the contract's, rather than debugging an opaque
    /// `InvalidProof` revert.
    function computePublicSignals(IPrivacyPool.Operation calldata op) public view returns (uint256[N_PUB] memory) {
        bytes32 oldRoot = ring.getRoot(op.startAggregationIndex);
        bytes32 startAggregationHash = ring.getHash(op.startAggregationIndex);
        bytes32 endAggregationHash = ring.getHash(op.endAggregationIndex);

        return ProofLib.toPublicSignals(oldRoot, startAggregationHash, endAggregationHash, op);
    }

    /// @notice Verifies that the provided operation is valid or reverts if not.
    function verifyOperation(IPrivacyPool.Operation calldata op) public view {
        // Verify the ring has room for this operation's output commitments
        uint128 outputs;
        for (uint256 i; i < N_OUTPUTS; ++i) {
            if (op.commitmentsOut[i] != 0) ++outputs;
        }
        ring.requireSpace(outputs);

        // Verify the ring can be advanced to this operation's end index.
        ring.requireAdvanceable(op.endAggregationIndex, op.newRoot);

        // Verify nullifier uniqueness & unspentness
        ProofLib._requireUnique(op.nullifiers);
        for (uint256 i; i < N_INPUTS; ++i) {
            bytes32 hash = op.nullifiers[i];
            _requireUnspent(hash);
        }

        // Verify the zk proof
        uint256[N_PUB] memory pubSignals = computePublicSignals(op);
        if (!VERIFIER.verifyProof(op.proof.pA, op.proof.pB, op.proof.pC, pubSignals)) {
            revert InvalidProof();
        }

        // Verify spendability
        address[N_INPUTS] memory spendabilityAddresses = ProofLib.spendabilityAddresses(op);
        for (uint256 i; i < N_INPUTS; ++i) {
            if (spendabilityAddresses[i] == address(0)) continue;
            ISpendability(spendabilityAddresses[i]).requireSpendable(op);
        }
    }

    // -------------------- INTERNAL STATE-CHANGING --------------------

    /// @notice Executes the state changes specified by the operation.
    /// @dev Assumes the operation has already been verified.
    function _executeOperation(IPrivacyPool.Operation calldata op) internal {
        uint128 tailBefore = ring.latestRootIndex();
        ring.advance(op.endAggregationIndex, op.newRoot);
        if (ring.latestRootIndex() != tailBefore) {
            emit AggregationAdvanced(op.endAggregationIndex, op.newRoot);
        }

        // Nullify the input notes
        for (uint256 i; i < N_INPUTS; ++i) {
            bytes32 hash = op.nullifiers[i];
            _spend(hash);
            emit Nullified(hash);
        }

        // Stage any output commitments
        for (uint256 i; i < N_OUTPUTS; ++i) {
            bytes32 commitment = op.commitmentsOut[i];
            ring.stage(_hash, commitment);
            emit Committed(commitment, op.context.ciphertexts[i]);
        }

        // Execute any unshielding transfers
        for (uint256 i; i < N_WITHDRAWALS; ++i) {
            address asset = op.unshieldAssets[i];
            uint128 amount = op.unshieldAmounts[i];
            address recipient = op.context.unshieldRecipients[i];
            if (amount == 0) continue;
            IERC20(asset).safeTransfer(recipient, amount);
            emit Withdrawn(asset, amount, recipient);
        }
    }

    // -------------------- INTERNAL VIEW --------------------

    /// @dev Overridable in test harnesses to swap out the hash function.
    function _hash(bytes32 prevHash, bytes32 commitment) internal pure virtual returns (bytes32) {
        return bytes32(LibPoseidon2T2_BN254.compress(uint256(prevHash), uint256(commitment), 0));
    }
}

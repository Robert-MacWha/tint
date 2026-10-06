// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SafeERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {IVerifier} from "./interfaces/IVerifier.sol";
import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {ISpendability} from "./interfaces/ISpendability.sol";
import {N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, N_PUB, N_COMPRESSED_PUB} from "./lib/Constants.sol";
import {ProofLib} from "./lib/ProofLib.sol";
import {LibPoseidon2T3_BN254} from "./lib/LibPoseidon2T3_BN254.sol";
import {NullifierRegistry} from "./NullifierRegistry.sol";
import {LibSkewMmrWithHistory} from "./lib/LibSkewMmrWithHistory.sol";
import {LibSkewMmr} from "./lib/LibSkewMmr.sol";

/// @notice Privacy-preserving token pool
contract Tint is IPrivacyPool, NullifierRegistry {
    using SafeERC20 for IERC20;
    using LibSkewMmrWithHistory for LibSkewMmrWithHistory.State;
    using LibSkewMmr for LibSkewMmr.State;

    IVerifier public immutable VERIFIER;

    LibSkewMmrWithHistory.State internal mmr;

    event Deposited(bytes32 commitment, address indexed asset, uint128 amount, bytes encryptedPartial);
    event Committed(bytes32 commitment, bytes encryptedNote);
    event Nullified(bytes32 nullifier);
    event Withdrawn(address indexed asset, uint128 amount, address indexed recipient);

    error InvalidFrontier();
    error InvalidProof();

    constructor(IVerifier _verifier) {
        VERIFIER = _verifier;
        mmr.prewarm();
    }

    // -------------------- EXTERNAL STATE-CHANGING --------------------

    /// @notice Deposits an asset into the pool, appending the commitment to the set.
    ///
    /// @param asset The ERC20 token contract address.
    /// @param amount The amount to deposit in.
    /// @param partialCommitment The partial commitment for the private output note.
    ///
    /// @dev The caller must have approved this contract to spend at least `amount` of `asset`.
    function deposit(address asset, uint128 amount, bytes32 partialCommitment, bytes calldata encryptedPartial)
        external
    {
        bytes32 commitment = ProofLib.toCommitment(asset, amount, partialCommitment);
        mmr.append(commitment, _hash);
        IERC20(asset).safeTransferFrom(msg.sender, address(this), amount);
        emit Deposited(commitment, asset, amount, encryptedPartial);
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

    /// @notice Verifies a given frontier against the MMR state.
    function verifyFrontier(uint256 histState, bytes32[] calldata _frontier) external view returns (bool) {
        return mmr.verifyFrontier(histState, _frontier);
    }

    /// @notice Returns the packed MMR state word: ranks, depth and count.
    /// @dev Pass this as `Operation.histState` alongside `frontier()`.
    function mmrState() external view returns (uint256) {
        return mmr.mmr.state;
    }

    /// @notice Returns the number of trees in the frontier.
    function mmrDepth() external view returns (uint256) {
        return mmr.mmr.depth();
    }

    /// @notice Returns the total number of commitments ever appended.
    function mmrCount() external view returns (uint256) {
        return mmr.mmr.count();
    }

    /// @notice Returns the root of the tree at `tree`, largest-ranked first.
    function mmrRoot(uint256 tree) external view returns (bytes32) {
        return mmr.mmr.roots[tree];
    }

    /// @notice Returns the rank of the tree at `tree`.
    function mmrRank(uint256 tree) external view returns (uint8) {
        return mmr.mmr.ranks(tree);
    }

    /// @notice Returns the live frontier, ready to drop into an `Operation`.
    function frontier() external view returns (bytes32[] memory roots) {
        uint256 depth = mmr.mmr.depth();
        roots = new bytes32[](depth);
        for (uint256 i = 0; i < depth; ++i) {
            roots[i] = mmr.mmr.roots[i];
        }
    }

    /// @notice Computes the Groth16 public-signal vector `op` must satisfy.
    /// Exposed so a client can cross-check its locally-computed proof inputs
    /// against the contract's, rather than debugging an opaque
    /// `InvalidProof` revert.
    function computePublicSignals(IPrivacyPool.Operation calldata op) public pure returns (uint256[N_PUB] memory) {
        return ProofLib.toPublicSignals(op);
    }

    /// @notice Verifies that the provided operation is valid or reverts if not.
    function verifyOperation(IPrivacyPool.Operation calldata op) public view {
        // Verify the frontier the proof was built against is one this pool committed to.
        // Must come first: nothing else may read `op.frontier` until it is authenticated.
        if (!mmr.verifyFrontier(op.histState, op.frontier)) revert InvalidFrontier();

        // Verify nullifier uniqueness & unspentness
        ProofLib._requireUnique(op.nullifiers);
        for (uint256 i; i < N_INPUTS; ++i) {
            bytes32 hash = op.nullifiers[i];
            _requireUnspent(hash);
        }

        // Verify the zk proof. The verifier itself only checks the 3
        // hybrid-compression signals; `pubSignals` is still fully
        // reconstructed above so `toCompressedSignals` can bind them to it.
        uint256[N_PUB] memory pubSignals = computePublicSignals(op);
        uint256[N_COMPRESSED_PUB] memory compressedSignals = ProofLib.toCompressedSignals(pubSignals, op.beta);
        try VERIFIER.verify(op.proof.proof, compressedSignals) {}
        catch {
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
        // Nullify the input notes
        for (uint256 i; i < N_INPUTS; ++i) {
            bytes32 hash = op.nullifiers[i];
            _spend(hash);
            emit Nullified(hash);
        }

        // Append any output commitments. Zero is the padding value for an unused
        // output slot; appending it would put a junk element into the set.
        for (uint256 i; i < N_OUTPUTS; ++i) {
            bytes32 commitment = op.commitmentsOut[i];
            if (commitment == 0) continue;
            mmr.append(commitment, _hash);
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
    function _hash(bytes32 a, bytes32 b, bytes32 c) internal pure virtual returns (bytes32) {
        return bytes32(LibPoseidon2T3_BN254.compress(uint256(a), uint256(b), uint256(c), 0));
    }
}

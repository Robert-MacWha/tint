// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SafeERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {IVerifier} from "./interfaces/IVerifier.sol";
import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {IPolicy} from "./interfaces/IPolicy.sol";
import {IPoseidon2T3} from "./interfaces/IPoseidon2T3.sol";
import {
    MMR_MAX_DEPTH,
    MMR_HISTORY_SIZE,
    N_INPUTS,
    N_OUTPUTS,
    N_WITHDRAWALS,
    N_COMPRESSED_PUB
} from "./lib/Constants.sol";
import {ProofLib} from "./lib/ProofLib.sol";
import {NullifierRegistry} from "./NullifierRegistry.sol";
import {Simulatable} from "./Simulatable.sol";
import {SkewMmrWithHistory} from "@skew-mmr/contracts/SkewMmrWithHistory.sol";

/// @notice Privacy-preserving token pool
contract Tint is IPrivacyPool, NullifierRegistry, SkewMmrWithHistory, Simulatable {
    using ProofLib for IPrivacyPool.Operation;
    using SafeERC20 for IERC20;

    IVerifier public immutable VERIFIER;
    IPoseidon2T3 public immutable POSEIDON;

    event Deposited(
        bytes32 commitment, uint64 leafIndex, address indexed asset, uint128 amount, bytes encryptedPartial
    );
    event Committed(bytes32 commitment, uint64 leafIndex, bytes encryptedNote);
    event Nullified(bytes32 nullifier);
    event Withdrawn(address indexed asset, uint128 amount, address indexed recipient);

    error InvalidFrontier();
    error InvalidProof();
    error InvalidCaller();

    constructor(IVerifier _verifier, IPoseidon2T3 _poseidon) SkewMmrWithHistory(MMR_MAX_DEPTH, MMR_HISTORY_SIZE) {
        VERIFIER = _verifier;
        POSEIDON = _poseidon;
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
        bytes32 commitment =
            bytes32(POSEIDON.compress(uint256(uint160(asset)), uint256(amount), uint256(partialCommitment), 0));
        uint64 leafIndex = mmr.count;
        _append(commitment);
        IERC20(asset).safeTransferFrom(msg.sender, address(this), amount);
        emit Deposited(commitment, leafIndex, asset, amount, encryptedPartial);
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
        bytes32 operationHash = operation.toOperationHash();
        assembly {
            tstore(slot, operationHash)
        }
    }

    /// @notice Executes a pre-verified operation.
    ///
    /// @dev Assuming the operation has been pre-verified and that no erc20
    /// transfers revert, this function is guaranteed to not revert.
    function executePreVerified(bytes32 slot, IPrivacyPool.Operation calldata operation) public {
        bytes32 operationHash = operation.toOperationHash();
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

    /// @notice Returns the MMR's live frontier.
    function frontier() external view returns (bytes32[] memory live) {
        live = new bytes32[](mmr.depth);
        for (uint256 i = 0; i < live.length; ++i) {
            live[i] = mmr.roots[i];
        }
    }

    /// @notice Verifies that the provided operation is valid or reverts if not.
    function verifyOperation(IPrivacyPool.Operation calldata op) public view {
        // Verify the operation is executed by its caller, if it names one.
        address caller = op.context.caller;
        if (caller != address(0) && caller != msg.sender) revert InvalidCaller();

        // Verify the frontier the proof was built against is one this pool committed to.
        (uint256 histRanks, uint256 histDepth, uint256 histCount, bytes32[] calldata histFrontier) = op.frontierState();
        if (!verifyFrontier(histRanks, histDepth, histCount, histFrontier)) revert InvalidFrontier();

        // Verify nullifier unspentness. Uniqueness is enforced in-circuit.
        for (uint256 i; i < N_INPUTS; ++i) {
            bytes32 hash = op.nullifier(i);
            _requireUnspent(hash);
        }

        // Verify the zk proof.
        uint256[N_COMPRESSED_PUB] memory compressedSignals = op.toCompressedSignals();
        try VERIFIER.verify(op.proof.proof, compressedSignals) {}
        catch {
            if (!_simulating()) revert InvalidProof();
        }

        // Verify policy
        address[N_INPUTS] memory policyAddresses = op.policyAddresses();
        for (uint256 i; i < N_INPUTS; ++i) {
            if (policyAddresses[i] == address(0)) continue;
            try IPolicy(policyAddresses[i]).requireAuthorized(op) {}
            catch (bytes memory reason) {
                if (!_simulating()) _bubble(reason);
            }
        }
    }

    // -------------------- INTERNAL STATE-CHANGING --------------------

    /// @notice Executes the state changes specified by the operation.
    /// @dev Assumes the operation has already been verified.
    function _executeOperation(IPrivacyPool.Operation calldata op) internal {
        // Nullify the non-sentinel input notes
        for (uint256 i; i < N_INPUTS; ++i) {
            bytes32 hash = op.nullifier(i);
            if (hash == bytes32(0)) continue;
            _spend(hash);
            emit Nullified(hash);
        }

        // Append the non-sentinel output notes
        for (uint256 i; i < N_OUTPUTS; ++i) {
            bytes32 commitment = op.commitmentOut(i);
            if (commitment == 0) continue;
            uint64 leafIndex = mmr.count;
            _append(commitment);
            emit Committed(commitment, leafIndex, op.context.ciphertexts[i]);
        }

        // Execute the non-sentinel withdrawals
        for (uint256 i; i < N_WITHDRAWALS; ++i) {
            address asset = op.unshieldAsset(i);
            uint128 amount = op.unshieldAmount(i);
            address recipient = op.unshieldRecipient(i);
            if (amount == 0) continue;
            IERC20(asset).safeTransfer(recipient, amount);
            emit Withdrawn(asset, amount, recipient);
        }
    }

    // -------------------- INTERNAL VIEW --------------------

    /// @dev Overridable in test harnesses to swap out the hash function.
    function _hash(bytes32 a, bytes32 b, bytes32 c) internal view virtual override returns (bytes32) {
        return bytes32(POSEIDON.compress(uint256(a), uint256(b), uint256(c), 0));
    }

    /// @dev Re-raises a caught revert.
    function _bubble(bytes memory reason) internal pure {
        assembly {
            revert(add(reason, 0x20), mload(reason))
        }
    }
}

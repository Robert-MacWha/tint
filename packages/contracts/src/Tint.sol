// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {
    InternalLeanIMT,
    LeanIMTData
} from "@zk-kit/imt.sol/internal/InternalLeanIMT.sol";
import {
    SafeERC20,
    IERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IVerifier} from "./IVerifier.sol";
import {IArchiveVerifier} from "./IArchiveVerifier.sol";
import {ISpendabilityVerifier} from "./ISpendabilityVerifier.sol";

/// @notice Privacy-preserving token pool. Users shield ERC-20 tokens into a note-based
/// system, transact privately via ZK proofs, and unshield back to plain ERC-20.
///
/// Asset IDs in the circuit are token addresses cast to uint256 — no registry needed.
///
/// Dummy inputs: nullifiers_[i] == 0 marks a slot as unused; the circuit skips its
/// merkle proof and the contract skips nullifier recording for that slot.
///
/// Self-spendability: when spendabilityAddresses[i] == address(this), the corresponding
/// spendabilityData[i] is ABI-decoded as (bytes32 vkeyHash, pA, pB, pC) and the
/// embedded sub-proof is verified against a registered spendability verifier.
contract Tint {
    using SafeERC20 for IERC20;

    // Circuit parameters — must match the deployed verifier.
    uint256 private constant N_INPUTS = 6;
    uint256 private constant N_OUTPUTS = 6;

    // Total public signals: stagingRoot(1) + archiveRoot(1) + N_INPUTS*2 + N_OUTPUTS*5 = 44
    uint256 private constant N_PUB = 44;

    // Masks keccak256 output to 252 bits, always within the BN254 scalar field.
    uint256 private constant FIELD_MASK = (1 << 252) - 1;

    IVerifier public immutable verifier;
    IArchiveVerifier public immutable archiveVerifier;
    address public immutable owner;

    uint256 public archiveRoot;

    mapping(uint256 => bool) public spent;
    mapping(bytes32 => ISpendabilityVerifier) public vkeys;

    LeanIMTData private _staging;

    event Shielded(address indexed token, uint256 amount, uint256 commitment);
    event Nullified(uint256 indexed nullifier);
    event Committed(uint256 indexed commitment);
    event Unshielded(
        address indexed token,
        address indexed recipient,
        uint256 amount
    );
    event Archived(uint256 indexed newArchiveRoot, uint256 indexed stagingRoot);

    constructor(address _verifier, address _archiveVerifier) {
        verifier = IVerifier(_verifier);
        archiveVerifier = IArchiveVerifier(_archiveVerifier);
        owner = msg.sender;
    }

    function registerVkey(bytes32 vkeyHash, address verifierAddr) external {
        require(msg.sender == owner, "Only owner");
        vkeys[vkeyHash] = ISpendabilityVerifier(verifierAddr);
    }

    function stagingRoot() external view returns (uint256) {
        return InternalLeanIMT._root(_staging);
    }

    /// @notice Deposit tokens and register a commitment in the staging tree.
    function shield(
        address token,
        uint256 amount,
        uint256 commitment
    ) external {
        IERC20(token).safeTransferFrom(msg.sender, address(this), amount);
        InternalLeanIMT._insert(_staging, commitment);
        emit Shielded(token, amount, commitment);
    }

    /// @notice Insert the current staging root into the archive tree and reset staging.
    /// The proof must demonstrate newArchiveRoot = LeanIMT.insert(archiveRoot, stagingRoot).
    function archive(
        uint256[2] calldata pA,
        uint256[2][2] calldata pB,
        uint256[2] calldata pC,
        uint256 newArchiveRoot
    ) external {
        require(_staging.size > 0, "Nothing to archive");

        uint256 currentStagingRoot = InternalLeanIMT._root(_staging);

        uint[3] memory pub;
        pub[0] = archiveRoot;
        pub[1] = currentStagingRoot;
        pub[2] = newArchiveRoot;
        require(archiveVerifier.verifyProof(pA, pB, pC, pub), "Invalid archive proof");

        archiveRoot = newArchiveRoot;
        _resetStaging();

        emit Archived(newArchiveRoot, currentStagingRoot);
    }

    /// @notice Process a private transaction: verify proof, nullify inputs,
    /// insert output commitments into staging, and execute any unshields.
    /// Slots with nullifiers_[i] == 0 are treated as dummy (unused) inputs.
    function transact(
        uint256[2] calldata pA,
        uint256[2][2] calldata pB,
        uint256[2] calldata pC,
        uint256[N_INPUTS] calldata nullifiers_,
        uint256[N_OUTPUTS] calldata commitmentsOut,
        uint256[N_OUTPUTS] calldata unshieldRecipients,
        uint256[N_OUTPUTS] calldata unshieldAmounts,
        uint256[N_OUTPUTS] calldata unshieldAssets,
        address[N_INPUTS] calldata spendabilityAddresses,
        bytes[N_INPUTS] calldata spendabilityData
    ) external {
        _verifyProof(
            pA, pB, pC,
            nullifiers_, commitmentsOut,
            unshieldRecipients, unshieldAmounts, unshieldAssets,
            spendabilityAddresses, spendabilityData
        );
        _checkSelfSpendability(nullifiers_, commitmentsOut, spendabilityAddresses, spendabilityData);
        _nullify(nullifiers_);
        _processOutputs(commitmentsOut, unshieldRecipients, unshieldAmounts, unshieldAssets);
    }

    function _verifyProof(
        uint256[2] calldata pA,
        uint256[2][2] calldata pB,
        uint256[2] calldata pC,
        uint256[N_INPUTS] calldata nullifiers_,
        uint256[N_OUTPUTS] calldata commitmentsOut,
        uint256[N_OUTPUTS] calldata unshieldRecipients,
        uint256[N_OUTPUTS] calldata unshieldAmounts,
        uint256[N_OUTPUTS] calldata unshieldAssets,
        address[N_INPUTS] calldata spendabilityAddresses,
        bytes[N_INPUTS] calldata spendabilityData
    ) private view {
        // Public signals layout (44 total):
        //   [0]      stagingRoot
        //   [1]      archiveRoot
        //   [2..7]   nullifiers[0..5]
        //   [8..13]  commitmentsOut[0..5]
        //   [14..19] unshieldRecipients[0..5]
        //   [20..25] unshieldAmounts[0..5]
        //   [26..31] unshieldAssets[0..5]
        //   [32..37] spendabilityAddresses[0..5]
        //   [38..43] spendabilityData hashes[0..5]
        uint[N_PUB] memory pub;
        pub[0] = InternalLeanIMT._root(_staging);
        pub[1] = archiveRoot;
        for (uint256 i; i < N_INPUTS; ++i)  pub[2  + i] = nullifiers_[i];
        for (uint256 i; i < N_OUTPUTS; ++i) pub[8  + i] = commitmentsOut[i];
        for (uint256 i; i < N_OUTPUTS; ++i) pub[14 + i] = unshieldRecipients[i];
        for (uint256 i; i < N_OUTPUTS; ++i) pub[20 + i] = unshieldAmounts[i];
        for (uint256 i; i < N_OUTPUTS; ++i) pub[26 + i] = unshieldAssets[i];
        for (uint256 i; i < N_INPUTS; ++i)  pub[32 + i] = uint256(uint160(spendabilityAddresses[i]));
        for (uint256 i; i < N_INPUTS; ++i)  pub[38 + i] = uint256(keccak256(spendabilityData[i])) & FIELD_MASK;
        require(verifier.verifyProof(pA, pB, pC, pub), "Invalid proof");
    }

    function _checkSelfSpendability(
        uint256[N_INPUTS] calldata nullifiers_,
        uint256[N_OUTPUTS] calldata commitmentsOut,
        address[N_INPUTS] calldata spendabilityAddresses,
        bytes[N_INPUTS] calldata spendabilityData
    ) private view {
        for (uint256 i; i < N_INPUTS; ++i) {
            if (spendabilityAddresses[i] != address(this)) continue;

            (
                bytes32 vkeyHash,
                uint[2] memory spA,
                uint[2][2] memory spB,
                uint[2] memory spC
            ) = abi.decode(spendabilityData[i], (bytes32, uint[2], uint[2][2], uint[2]));

            ISpendabilityVerifier sv = vkeys[vkeyHash];
            require(address(sv) != address(0), "Unknown vkey");

            uint[7] memory pub;
            pub[0] = nullifiers_[i];
            for (uint256 j; j < N_OUTPUTS; ++j) pub[1 + j] = commitmentsOut[j];

            require(sv.verifyProof(spA, spB, spC, pub), "Invalid spendability proof");
        }
    }

    function _nullify(uint256[N_INPUTS] calldata nullifiers_) private {
        for (uint256 i; i < N_INPUTS; ++i) {
            if (nullifiers_[i] == 0) continue; // dummy input slot
            require(!spent[nullifiers_[i]], "Nullifier already spent");
            spent[nullifiers_[i]] = true;
            emit Nullified(nullifiers_[i]);
        }
    }

    function _processOutputs(
        uint256[N_OUTPUTS] calldata commitmentsOut,
        uint256[N_OUTPUTS] calldata unshieldRecipients,
        uint256[N_OUTPUTS] calldata unshieldAmounts,
        uint256[N_OUTPUTS] calldata unshieldAssets
    ) private {
        for (uint256 i; i < N_OUTPUTS; ++i) {
            if (commitmentsOut[i] != 0) {
                InternalLeanIMT._insert(_staging, commitmentsOut[i]);
                emit Committed(commitmentsOut[i]);
            } else if (unshieldRecipients[i] != 0) {
                address token = address(uint160(unshieldAssets[i]));
                address recipient = address(uint160(unshieldRecipients[i]));
                IERC20(token).safeTransfer(recipient, unshieldAmounts[i]);
                emit Unshielded(token, recipient, unshieldAmounts[i]);
            }
        }
    }

    /// @dev Resets the staging tree to empty. Safe because InternalLeanIMT always
    /// writes sideNodes[k] before reading it during sequential insertion — stale
    /// values from the previous cycle are overwritten before first use.
    /// The one exception is the root (sideNodes[depth]), which is explicitly cleared.
    function _resetStaging() private {
        _staging.sideNodes[_staging.depth] = 0;
        _staging.size = 0;
        _staging.depth = 0;
    }
}

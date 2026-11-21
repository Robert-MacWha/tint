// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {IPolicy} from "../../interfaces/IPolicy.sol";
import {IPrivacyPool} from "../../interfaces/IPrivacyPool.sol";
import {IPoseidon2T2} from "../../interfaces/IPoseidon2T2.sol";
import {ProofLib} from "../../lib/ProofLib.sol";
import {N_INPUTS} from "../../lib/Constants.sol";
import {Verifier} from "../../codegen/HardwareWalletVerifier.sol";

/// @notice Single-signer policy rule for hardware wallets. Verifies a proof
/// that a private secp256k1 signer signed the EIP-712
/// `TintOperation(bytes32 signingHash)` message for the operation.
contract HardwareWalletPolicy is IPolicy, EIP712 {
    using ProofLib for IPrivacyPool.Operation;

    /// @dev Mirrors `TintOperation` in `hardware_wallet.rs`.
    bytes32 public constant OPERATION_TYPEHASH = keccak256("TintOperation(bytes32 signingHash)");

    Verifier public immutable VERIFIER;
    IPoseidon2T2 public immutable POSEIDON;

    error NoInputsForThisPolicy();

    constructor(Verifier verifier, IPoseidon2T2 poseidon) EIP712("Tint Hardware Wallet Policy", "1") {
        VERIFIER = verifier;
        POSEIDON = poseidon;
    }

    function requireAuthorized(IPrivacyPool.Operation calldata operation) external view {
        for (uint256 i = 0; i < N_INPUTS; i++) {
            if (operation.policyAddress(i) != address(this)) continue;
            bytes calldata proof = operation.context.policyInputs[i];
            uint256 signingHash = operation.signingHash(POSEIDON);
            uint256 digest = uint256(operationDigest(signingHash));
            uint256[4] memory pubSignals =
                [uint256(uint160(address(this))), signingHash, digest >> 128, uint256(uint128(digest))];

            VERIFIER.verifyProof(proof, pubSignals);
            return;
        }

        revert NoInputsForThisPolicy();
    }

    /// @notice The EIP-712 digest the signer signs to authorize an operation
    /// with `signingHash`.
    function operationDigest(uint256 signingHash) public view returns (bytes32) {
        return _hashTypedDataV4(keccak256(abi.encode(OPERATION_TYPEHASH, signingHash)));
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPoseidon2T2} from "../interfaces/IPoseidon2T2.sol";
import {LibHybridCompression} from "./LibHybridCompression.sol";
import {IPrivacyPool} from "../interfaces/IPrivacyPool.sol";
import {
    N_CONST,
    N_INPUTS,
    N_OUTPUTS,
    FRONTIER_OFFSET,
    N_PUB,
    N_COMPRESSED_PUB,
    BN254_FR_MODULUS
} from "./Constants.sol";

library ProofLib {
    /// @notice Groth16 proof structure.
    struct Proof {
        uint256[8] proof;
    }

    error InvalidPublicSignals();

    /// @notice Expands `op.pub` into the full `N_PUB`-length Groth16
    /// public-signal vector, matching the order `JoinSplitResultVar::flatten`
    /// allocates in.
    ///
    /// @dev Reverts if `op.pub[0]` is not `op.context`'s bound-params hash.
    function toPublicSignals(IPrivacyPool.Operation calldata op) internal pure returns (uint256[] memory stmt) {
        bytes32[] calldata pub = op.pub;
        if (pub.length > N_PUB) revert InvalidPublicSignals();
        if (uint256(pub[0]) != toBoundParamsHash(op)) revert InvalidPublicSignals();

        // `new` zero-fills, which supplies the frontier's padding.
        stmt = new uint256[](N_PUB);
        assembly ("memory-safe") {
            calldatacopy(add(stmt, 0x20), pub.offset, shl(5, pub.length))
        }
    }

    /// @notice Compresses `op`'s `N_PUB`-length public-signal vector down to
    /// the `N_COMPRESSED_PUB` (`alpha`, `beta`, `gamma`) values the Groth16
    /// verifier actually checks, reconstructing `alpha`/`gamma` from the
    /// prover-supplied `op.beta` and the plaintext signals.
    function toCompressedSignals(IPrivacyPool.Operation calldata op)
        internal
        pure
        returns (uint256[N_COMPRESSED_PUB] memory)
    {
        uint256 beta = op.beta;
        (uint256 alpha, uint256 gamma) =
            LibHybridCompression.hybridCompression(beta, toPublicSignals(op), BN254_FR_MODULUS);
        return [alpha, beta, gamma];
    }

    /// @notice Computes the operation hash, used to uniquely identify an operation.
    function toOperationHash(IPrivacyPool.Operation calldata op) internal pure returns (bytes32) {
        return keccak256(abi.encode(op));
    }

    /// @notice Computes the hash of the bound parameters in an operation.
    function toBoundParamsHash(IPrivacyPool.Operation calldata op) internal pure returns (uint256) {
        return uint256(keccak256(abi.encode(op.context))) % BN254_FR_MODULUS;
    }

    /// @notice Computes the message spending policies authorize:
    /// `poseidon2(operationHash, contextHash)`. Covers everything in the
    /// operation except the policy inputs themselves.
    /// @dev Mirrors `OperationWitness::signing_hash`.
    function signingHash(IPrivacyPool.Operation calldata op, IPoseidon2T2 poseidon) internal view returns (uint256) {
        return poseidon.compress(uint256(operationHash(op)), contextHash(op), 0);
    }

    /// @notice Hashes the context fields the operation hash omits, except
    /// policy inputs.
    /// @dev Mirrors `OperationWitness::context_hash`.
    function contextHash(IPrivacyPool.Operation calldata op) internal pure returns (uint256) {
        return uint256(keccak256(abi.encode(op.context.caller, op.context.binding, op.context.ciphertexts)))
            % BN254_FR_MODULUS;
    }

    function frontierState(IPrivacyPool.Operation calldata op)
        internal
        pure
        returns (uint256 ranks, uint256 depth, uint256 count, bytes32[] calldata frontier)
    {
        bytes32[] calldata pub = op.pub;
        return (uint256(pub[2]), uint256(pub[3]), uint256(pub[4]), pub[FRONTIER_OFFSET:]);
    }

    function operationHash(IPrivacyPool.Operation calldata op) internal pure returns (bytes32) {
        return op.pub[1];
    }

    function nullifier(IPrivacyPool.Operation calldata op, uint256 i) internal pure returns (bytes32) {
        return op.pub[N_CONST + 2 * i];
    }

    function policyAddress(IPrivacyPool.Operation calldata op, uint256 i) internal pure returns (address) {
        return _toAddress(op.pub[N_CONST + 2 * i + 1]);
    }

    function commitmentOut(IPrivacyPool.Operation calldata op, uint256 i) internal pure returns (bytes32) {
        return op.pub[N_CONST + 2 * N_INPUTS + i];
    }

    function unshieldAmount(IPrivacyPool.Operation calldata op, uint256 i) internal pure returns (uint128) {
        uint256 amount = uint256(op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i]);
        if (amount > type(uint128).max) revert InvalidPublicSignals();
        // forge-lint: disable-next-line(unsafe-typecast)
        return uint128(amount);
    }

    function unshieldAsset(IPrivacyPool.Operation calldata op, uint256 i) internal pure returns (address) {
        return _toAddress(op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i + 1]);
    }

    function unshieldRecipient(IPrivacyPool.Operation calldata op, uint256 i) internal pure returns (address) {
        return _toAddress(op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i + 2]);
    }

    /// @notice Returns the unique non-zero policy addresses in an operation, zero-padded.
    function policyAddresses(IPrivacyPool.Operation calldata op)
        internal
        pure
        returns (address[N_INPUTS] memory output)
    {
        uint256 count = 0;

        for (uint256 i = 0; i < N_INPUTS; i++) {
            address addr = policyAddress(op, i);
            if (addr == address(0)) continue;
            bool exists = false;
            for (uint256 j = 0; j < count; j++) {
                if (output[j] == addr) {
                    exists = true;
                    break;
                }
            }
            if (!exists) {
                output[count] = addr;
                count++;
            }
        }
    }

    function _toAddress(bytes32 x) private pure returns (address) {
        if (uint256(x) > type(uint160).max) revert InvalidPublicSignals();
        return address(uint160(uint256(x)));
    }
}

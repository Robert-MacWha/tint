// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {LibPoseidon2Yul} from "poseidon2-evm/src/bn254/yul/LibPoseidon2Yul.sol";
import {
    N_INPUTS,
    N_OUTPUTS,
    N_WITHDRAWALS,
    N_PUB,
    BN254_FR_MODULUS
} from "./Constants.sol";

library ProofLib {
    /// @notice Groth16 proof structure.
    struct Proof {
        uint256[2] pA;
        uint256[2][2] pB;
        uint256[2] pC;
    }

    function toCommitment(
        address asset,
        uint128 amount,
        bytes32 partialCommitment
    ) internal pure returns (bytes32) {
        return
            bytes32(
                LibPoseidon2Yul.hash_3(
                    assetToFr(asset),
                    uint256(amount),
                    uint256(partialCommitment)
                )
            );
    }

    function toBoundParamsHash(
        address[N_WITHDRAWALS] memory unshieldRecipients
    ) internal pure returns (bytes32) {
        bytes memory packed;
        for (uint256 i = 0; i < N_WITHDRAWALS; i++) {
            packed = abi.encodePacked(packed, unshieldRecipients[i]);
        }
        return keccak256(packed);
    }

    /// @notice Builds the Groth16 public-signal vector, matching the order
    /// `JoinSplit::synthesize` allocates public gr1cs variables in: four
    /// asserted inputs, then the circuit's computed outputs.
    ///
    /// `oldRootLength` and `startAggregationHash` are trusted directly from
    /// the caller, not independently re-derived on-chain — `oldRootLength`
    /// is bound to `oldRoot` (already checked via `_validateOldRoot`) by the
    /// circuit's Merkle math, and `startAggregationHash` is bound to
    /// `endAggregationHash` (checked by the caller via `_getHash`, see
    /// `Tint.verifyOperation`) by Poseidon's preimage resistance — a false
    /// value for either could only produce a valid proof via a hash
    /// collision.
    function toPublicSignals(
        bytes32 oldRoot,
        uint64 oldRootLength,
        bytes32 startAggregationHash,
        bytes32 boundParamsHash,
        bytes32 newRoot,
        bytes32 endAggregationHash,
        bytes32[N_INPUTS] memory nullifiers,
        bytes32[N_OUTPUTS] memory commitmentsOut,
        uint128[N_WITHDRAWALS] memory unshieldAmounts,
        address[N_WITHDRAWALS] memory unshieldAssets
    ) internal pure returns (uint256[N_PUB] memory) {
        uint256[N_PUB] memory pub;
        pub[0] = uint256(oldRoot);
        pub[1] = uint256(oldRootLength);
        pub[2] = uint256(startAggregationHash);
        // Matches `Fr::from_be_bytes_mod_order` in tint-rs: a raw keccak256
        // digest can exceed the BN254 scalar field, so it must be reduced
        // mod r. (Poseidon outputs elsewhere in this array are already valid
        // field elements and don't need this.)
        pub[3] = uint256(boundParamsHash) % BN254_FR_MODULUS;
        pub[4] = uint256(newRoot);
        pub[5] = uint256(endAggregationHash);

        for (uint256 i = 0; i < N_INPUTS; i++) {
            pub[6 + i] = uint256(nullifiers[i]);
        }

        for (uint256 i = 0; i < N_OUTPUTS; i++) {
            pub[6 + N_INPUTS + i] = uint256(commitmentsOut[i]);
        }

        for (uint256 i = 0; i < N_WITHDRAWALS; i++) {
            pub[6 + N_INPUTS + N_OUTPUTS + 2 * i] = unshieldAmounts[i];
            pub[6 + N_INPUTS + N_OUTPUTS + 2 * i + 1] = assetToFr(
                unshieldAssets[i]
            );
        }

        return pub;
    }

    /// @notice Matches `note::commitment::asset_to_fr` in tint-rs: the raw
    /// 20 address bytes, interpreted as a little-endian integer (not the
    /// natural big-endian byte order) and reduced mod the field order (a
    /// no-op here, since 160 bits never exceeds the field size).
    ///
    /// TODO: Simplify into just a typecast
    function assetToFr(address asset) internal pure returns (uint256) {
        bytes20 addr = bytes20(asset);
        uint256 reversed;
        for (uint256 i = 0; i < 20; i++) {
            reversed = (reversed << 8) | uint8(addr[19 - i]);
        }
        return reversed;
    }
}

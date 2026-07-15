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
                    uint256(uint160(asset)),
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
    /// `JoinSplit::synthesize` allocates public gr1cs variables in.
    function toPublicSignals(
        bytes32 oldRoot,
        uint128 startAggregationIndex,
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
        pub[1] = uint256(startAggregationIndex);
        pub[2] = uint256(startAggregationHash);
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
            pub[6 + N_INPUTS + N_OUTPUTS + 2 * i + 1] = uint256(
                uint160(unshieldAssets[i])
            );
        }

        return pub;
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {LibPoseidon2Yul} from "poseidon2-evm/src/bn254/yul/LibPoseidon2Yul.sol";
import {IPrivacyPool} from "../interfaces/IPrivacyPool.sol";
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

    /// @notice Builds the Groth16 public-signal vector, matching the order
    /// `JoinSplit::synthesize` allocates public gr1cs variables in.
    function toPublicSignals(
        bytes32 startAggregationHash,
        bytes32 endAggregationHash,
        IPrivacyPool.Operation calldata op
    ) internal pure returns (uint256[N_PUB] memory) {
        uint256[N_PUB] memory pub;
        pub[0] = uint256(op.oldRoot);
        pub[1] = uint256(op.startAggregationIndex);
        pub[2] = uint256(startAggregationHash);
        pub[3] = toBoundParamsHash(op);
        pub[4] = uint256(op.newRoot);
        pub[5] = uint256(endAggregationHash);

        for (uint256 i = 0; i < N_INPUTS; i++) {
            pub[6 + 2 * i] = uint256(op.nullifiers[i]);
            pub[6 + 2 * i + 1] = uint256(uint160(op.spendabilityAddresses[i]));
        }

        for (uint256 i = 0; i < N_OUTPUTS; i++) {
            pub[6 + 2 * N_INPUTS + i] = uint256(op.commitmentsOut[i]);
        }

        for (uint256 i = 0; i < N_WITHDRAWALS; i++) {
            pub[6 + 2 * N_INPUTS + N_OUTPUTS + 2 * i] = op.unshieldAmounts[i];
            pub[6 + 2 * N_INPUTS + N_OUTPUTS + 2 * i + 1] = uint256(
                uint160(op.unshieldAssets[i])
            );
        }

        return pub;
    }

    function toBoundParamsHash(
        IPrivacyPool.Operation calldata op
    ) internal pure returns (uint256) {
        bytes memory packed;
        for (uint256 i = 0; i < N_WITHDRAWALS; i++) {
            packed = abi.encodePacked(packed, op.unshieldRecipients[i]);
        }
        return uint256(keccak256(packed)) % BN254_FR_MODULUS;
    }
}

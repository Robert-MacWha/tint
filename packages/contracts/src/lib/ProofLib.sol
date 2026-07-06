// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {PoseidonT4} from "poseidon-solidity/PoseidonT4.sol";
import {N_INPUTS, N_OUTPUTS, N_PUB} from "./Constants.sol";

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
            PoseidonT4.poseidon(
                [
                    uint256(uint160(asset)),
                    uint256(amount),
                    uint256(partialCommitment)
                ]
            );
    }

    function toBoundParamsHash(
        address[N_OUTPUTS] memory spendabilityAddresses,
        bytes[N_OUTPUTS] memory spendabilityData,
        address[N_OUTPUTS] memory unshieldRecipients
    ) internal pure returns (bytes32) {
        bytes memory packed;
        for (uint256 i = 0; i < N_OUTPUTS; i++) {
            packed = abi.encodePacked(
                packed,
                spendabilityAddresses[i],
                spendabilityData[i],
                unshieldRecipients[i]
            );
        }
        return keccak256(packed);
    }

    function toPublicSignals(
        bytes32 oldRoot,
        bytes32 newRoot,
        bytes32 leavesAggregationHash,
        bytes32[N_INPUTS] memory nullifiers,
        bytes32[N_OUTPUTS] memory commitmentsOut,
        uint128[N_OUTPUTS] memory unshieldAmounts,
        address[N_OUTPUTS] memory unshieldAssets,
        bytes32 boundParamsHash
    ) internal pure returns (uint256[N_PUB] memory) {
        uint256[N_PUB] memory pub;
        pub[0] = uint256(oldRoot);
        pub[1] = uint256(newRoot);
        pub[2] = uint256(leavesAggregationHash);

        for (uint256 i = 0; i < N_INPUTS; i++) {
            pub[3 + i] = uint256(nullifiers[i]);
        }

        for (uint256 i = 0; i < N_OUTPUTS; i++) {
            pub[3 + N_INPUTS + i] = uint256(commitmentsOut[i]);
            pub[3 + N_INPUTS + N_OUTPUTS + i] = unshieldAmounts[i];
            pub[3 + N_INPUTS + 2 * N_OUTPUTS + i] = uint256(
                uint160(unshieldAssets[i])
            );
        }

        // pack the bound params into the last public signal
        pub[N_PUB - 1] = uint256(boundParamsHash);
        return pub;
    }
}

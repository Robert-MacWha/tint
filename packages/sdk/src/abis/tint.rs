use alloy::sol;

sol! {
    contract Tint {
        event Deposited(
            address indexed asset,
            uint128 amount,
            bytes32 partialCommitment,
        );
        event Committed(
            bytes32 indexed commitment
        );
        event Nullified(bytes32 indexed nullifier);
        event Withdrawn(
            address indexed asset,
            uint128 amount,
            address indexed recipient
        );
    }

    interface IPrivacyPool {
        struct Operation {
            bytes32 oldRoot;
            bytes32 newRoot;
            uint128 leavesAggregationIndex;
            bytes32[5] nullifiers;
            bytes32[5] commitmentsOut;
            uint128[5] unshieldAmounts;
            address[5] unshieldAssets;
            // bound params
            address[5] unshieldRecipients;
            address[5] spendabilityAddresses;
            bytes[5] spendabilityData;
            ProofLib.Proof proof;
        }

        function deposit(
            address asset,
            uint128 amount,
            bytes32 commitment
        ) external;

        function operate(Operation calldata operation) external;
    }

    library ProofLib {
        struct Proof {
            uint256[2] pA;
            uint256[2][2] pB;
            uint256[2] pC;
        }
    }
}

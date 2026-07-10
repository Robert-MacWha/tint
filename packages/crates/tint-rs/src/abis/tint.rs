use alloy_sol_macro::sol;
use ark_bn254::Bn254;
use ark_groth16::Proof;

sol! {
    contract Tint {
        event Deposited(
            address indexed asset,
            uint128 amount,
            bytes32 partialCommitment,
            bytes encrypted,
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

impl From<Proof<Bn254>> for ProofLib::Proof {
    fn from(p: Proof<Bn254>) -> Self {
        ProofLib::Proof {
            pA: [p.a.x.into(), p.a.y.into()],
            // TODO: Check if the order here is right - I think it should be swapped
            pB: [
                [p.b.x.c0.into(), p.b.x.c1.into()],
                [p.b.y.c0.into(), p.b.y.c1.into()],
            ],
            pC: [p.c.x.into(), p.c.y.into()],
        }
    }
}

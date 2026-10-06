use alloy_sol_macro::sol;
use ark_bn254::Bn254;
use ark_groth16::Proof;

sol!(
    uint128 constant MMR_MAX_DEPTH = 26;
    uint128 constant N_CONST = 3 + MMR_MAX_DEPTH;
    uint128 constant N_INPUTS = 5;
    uint128 constant N_OUTPUTS = 5;
    uint128 constant N_WITHDRAWALS = 2;
    uint128 constant N_PUB = N_CONST + 2 * N_INPUTS + N_OUTPUTS + 2 * N_WITHDRAWALS;

    #[derive(Debug)]
    library ProofLib {
        struct Proof {
            uint256[8] proof;
        }
    }

    #[derive(Debug)]
    interface IPrivacyPool {
        struct Operation {
            uint256 histState;
            bytes32[] frontier;
            bytes32 operationHash;
            bytes32[N_INPUTS] nullifiers;
            address[N_INPUTS] spendabilityAddresses;
            bytes32[N_OUTPUTS] commitmentsOut;
            uint128[N_WITHDRAWALS] unshieldAmounts;
            address[N_WITHDRAWALS] unshieldAssets;
            Context context;
            ProofLib.Proof proof;

            /// @dev Hybrid-compression challenge for `proof` (see
            /// `ProofLib.toCompressedSignals`).
            uint256 beta;
        }

        struct Context {
            bytes[N_INPUTS] spendabilityInputs;
            bytes[N_OUTPUTS] ciphertexts;
            address[N_WITHDRAWALS] unshieldRecipients;
        }
    }

    #[derive(Debug)]
    contract Tint {
        event Deposited(bytes32 commitment, address indexed asset, uint128 amount, bytes encryptedPartial);
        event Committed(bytes32 commitment, bytes encryptedNote);
        event Nullified(bytes32 nullifier);
        event Withdrawn(address indexed asset, uint128 amount, address indexed recipient);

        function deposit(address asset, uint128 amount, bytes32 partialCommitment, bytes calldata encryptedPartial) external;
        function operate(IPrivacyPool.Operation calldata operation) public;
        function preVerify(bytes32 slot, IPrivacyPool.Operation calldata operation) public;
        function executePreVerified(bytes32 slot, IPrivacyPool.Operation calldata operation) public;
        function latestRootIndex() external view returns (uint128);
        function verifyFrontier(uint256 histState, bytes32[] calldata _frontier) external view returns (bool);
        function computePublicSignals(IPrivacyPool.Operation calldata op) public view returns (uint256[N_PUB] memory);
        function verifyOperation(IPrivacyPool.Operation calldata op) public view;
    }
);

impl From<Proof<Bn254>> for ProofLib::Proof {
    fn from(p: Proof<Bn254>) -> Self {
        ProofLib::Proof {
            proof: [
                p.a.x.into(),
                p.a.y.into(),
                p.b.x.c1.into(),
                p.b.x.c0.into(),
                p.b.y.c1.into(),
                p.b.y.c0.into(),
                p.c.x.into(),
                p.c.y.into(),
            ],
        }
    }
}

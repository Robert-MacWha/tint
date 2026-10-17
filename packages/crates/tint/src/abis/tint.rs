use alloy_primitives::{B256, b256};
use alloy_sol_macro::sol;
use ark_bn254::Bn254;
use ark_groth16::Proof;

sol!(
    uint128 constant N_INPUTS = 5;
    uint128 constant N_OUTPUTS = 5;
    uint128 constant N_WITHDRAWALS = 2;

    #[derive(Debug)]
    library ProofLib {
        struct Proof {
            uint256[8] proof;
        }
    }

    #[derive(Debug)]
    interface IPrivacyPool {
        struct Operation {
            /// @dev Live prefix of the public-signal vector, `FRONTIER_OFFSET + histDepth` long.
            bytes32[] pub;
            Context context;
            ProofLib.Proof proof;

            /// @dev Hybrid-compression challenge for `proof` (see
            /// `ProofLib.toCompressedSignals`).
            uint256 beta;
        }

        struct Context {
            address caller;
            bytes32 binding;
            bytes[N_INPUTS] policyInputs;
            bytes[N_OUTPUTS] ciphertexts;
        }
    }

    #[derive(Debug)]
    contract Tint {
        event Deposited(bytes32 commitment, uint64 leafIndex, address indexed asset, uint128 amount, bytes encryptedPartial);
        event Committed(bytes32 commitment, uint64 leafIndex, bytes encryptedNote);
        event Nullified(bytes32 nullifier);
        event Withdrawn(address indexed asset, uint128 amount, address indexed recipient);

        function deposit(address asset, uint128 amount, bytes32 partialCommitment, bytes calldata encryptedPartial) external;
        function operate(IPrivacyPool.Operation calldata operation) public;
        function preVerify(bytes32 slot, IPrivacyPool.Operation calldata operation) public;
        function executePreVerified(bytes32 slot, IPrivacyPool.Operation calldata operation) public;
        function isSpent(bytes32 nullifierHash) external view returns (bool);
        function verifyFrontier(uint256 histRanks, uint256 histDepth, uint256 histCount, bytes32[] calldata frontier) external view returns (bool);
        function computePublicSignals(IPrivacyPool.Operation calldata op) public view returns (uint256[] memory);
        function verifyOperation(IPrivacyPool.Operation calldata op) public view;
    }
);

/// `Simulatable.sol`'s flag slot, at `erc7201:tint.simulation`. Nothing on-chain
/// writes it, so setting it with a state override lets an operation be
/// simulated without passing its proof and policy checks.
pub const SIMULATION_SLOT: B256 =
    b256!("0xb4a234b71e30e41b0c32030fc0a6277a036950341675d2266bc10f1fc28e5200");

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

#[cfg(test)]
mod tests {
    use alloy_primitives::{U256, keccak256};

    use super::*;

    /// `SIMULATION_SLOT` should be the ERC-7201 slot for `tint.simulation`.
    #[test]
    fn test_simulation_slot() {
        let namespace = U256::from_be_bytes(keccak256("tint.simulation").0) - U256::from(1);
        let slot = keccak256(namespace.to_be_bytes::<32>()) & !B256::with_last_byte(0xff);
        assert_eq!(slot, SIMULATION_SLOT);
    }
}

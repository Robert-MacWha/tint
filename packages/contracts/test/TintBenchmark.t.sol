// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {Groth16Verifier} from "../src/Groth16Verifier.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {
    AGGREGATION_RING_SIZE,
    N_PUB,
    N_INPUTS,
    N_OUTPUTS,
    N_WITHDRAWALS,
    GENESIS_ROOT,
    BN254_FR_MODULUS
} from "../src/lib/Constants.sol";

contract MockToken is ERC20 {
    constructor() ERC20("Mock", "MCK") {
        _mint(msg.sender, type(uint128).max);
    }
}

/// @notice Forwards to the real Groth16Verifier so proof verification pays
/// realistic pairing/precompile gas, but discards the result and always
/// reports success. A dummy all-zero proof takes the same EC-precompile
/// code path as a real one, so this is a close stand-in for a valid proof
/// without needing to generate one.
contract AlwaysTrueVerifier is IVerifier {
    Groth16Verifier public immutable INNER;

    constructor(Groth16Verifier _inner) {
        INNER = _inner;
    }

    function verifyProof(
        uint256[2] calldata pA,
        uint256[2][2] calldata pB,
        uint256[2] calldata pC,
        uint256[N_PUB] calldata pubSignals
    ) external view returns (bool) {
        INNER.verifyProof(pA, pB, pC, pubSignals);
        return true;
    }
}

contract TintHarness is Tint {
    constructor(address _verifier) Tint(_verifier) {}

    /// @dev Warms all storage slots touched by deposit() without calling deposit().
    /// This prevents warm-up writes from appearing in Forge's gas report for deposit().
    function warmStorage() external {
        for (uint256 i = 0; i < AGGREGATION_RING_SIZE; i++) {
            aggregationHashRing[i] = bytes32(uint256(i + 1));
        }
        totalStaged = AGGREGATION_RING_SIZE;
        totalConsumed = AGGREGATION_RING_SIZE;
    }
}

contract TintGasReportTest is Test {
    TintHarness public tint;
    MockToken public token;

    function setUp() public {
        token = new MockToken();
        Groth16Verifier groth16Verifier = new Groth16Verifier();
        AlwaysTrueVerifier verifier = new AlwaysTrueVerifier(groth16Verifier);
        tint = new TintHarness(address(verifier));
        token.approve(address(tint), type(uint256).max);
    }

    function test_shield_gas() public {
        tint.warmStorage();

        tint.deposit(address(token), 1, bytes32(uint256(1)), "");
    }

    function test_operate_gas() public {
        tint.warmStorage();
        token.transfer(address(tint), 1_000);

        IPrivacyPool.Operation memory op;
        op.oldRoot = GENESIS_ROOT;
        op.newRoot = bytes32(uint256(1));

        // Public signals fed to the verifier must be valid BN254 field
        // elements, or it rejects them before running the real pairing
        // check.
        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.nullifiers[i] = bytes32(
                uint256(keccak256(abi.encode("nullifier", i))) %
                    BN254_FR_MODULUS
            );
        }
        op.commitmentsOut[0] = bytes32(
            uint256(keccak256(abi.encode("commitment", uint256(0)))) %
                BN254_FR_MODULUS
        );
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        op.context.unshieldRecipients[0] = address(1);

        tint.operate(op);
    }

    function test_operate_full_gas() public {
        tint.warmStorage();
        token.transfer(address(tint), 1_000);

        IPrivacyPool.Operation memory op;
        op.oldRoot = GENESIS_ROOT;
        op.newRoot = bytes32(uint256(1));

        // Public signals fed to the verifier must be valid BN254 field
        // elements, or it rejects them before running the real pairing
        // check.
        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.nullifiers[i] = bytes32(
                uint256(keccak256(abi.encode("nullifier", i))) %
                    BN254_FR_MODULUS
            );
        }
        for (uint256 i = 0; i < N_OUTPUTS; i++) {
            op.commitmentsOut[i] = bytes32(
                uint256(keccak256(abi.encode("commitment", i))) %
                    BN254_FR_MODULUS
            );
        }
        for (uint256 i = 0; i < N_WITHDRAWALS; i++) {
            op.unshieldAmounts[i] = 1;
            op.unshieldAssets[i] = address(token);
            op.context.unshieldRecipients[i] = address(uint160(i + 1));
        }

        tint.operate(op);
    }
}

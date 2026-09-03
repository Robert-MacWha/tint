// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {TintVerifier} from "../src/TintVerifier.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ProofLib} from "../src/lib/ProofLib.sol";
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

/// @notice Forwards to the real Verifier so proof verification pays
/// realistic pairing/precompile gas, but discards the result and always
/// reports success. A dummy all-zero proof takes the same EC-precompile
/// code path as a real one, so this is a close stand-in for a valid proof
/// without needing to generate one.
contract AlwaysTrueVerifier is IVerifier {
    TintVerifier public immutable INNER;

    constructor(TintVerifier _inner) {
        INNER = _inner;
    }

    function verify(uint256[8] calldata proof, uint256[N_PUB] calldata pubSignals) external view {
        try INNER.verify(proof, pubSignals) {} catch {}
    }
}

contract TintHarness is Tint {
    constructor(IVerifier _verifier) Tint(_verifier) {}

    /// @dev Warms all storage slots touched by deposit() without calling deposit().
    /// This prevents warm-up writes from appearing in Forge's gas report for deposit().
    function warmStorage() external {
        for (uint256 i = 0; i < AGGREGATION_RING_SIZE; i++) {
            ring.buffer.buffer[i] = bytes32(uint256(i + 1));
        }
        ring.buffer.head = 0;
        ring.buffer.tail = 0;
    }

    function toPublicSignals(
        bytes32 oldRoot,
        bytes32 startAggregationHash,
        bytes32 endAggregationHash,
        IPrivacyPool.Operation calldata op
    ) external pure returns (uint256[N_PUB] memory) {
        return ProofLib.toPublicSignals(oldRoot, startAggregationHash, endAggregationHash, op);
    }
}

contract TintGasReportTest is Test {
    TintHarness public tint;
    MockToken public token;

    function setUp() public {
        token = new MockToken();
        TintVerifier groth16Verifier = new TintVerifier();
        AlwaysTrueVerifier verifier = new AlwaysTrueVerifier(groth16Verifier);
        tint = new TintHarness(verifier);
        token.approve(address(tint), type(uint256).max);
    }

    function test_shield_gas() public {
        tint.warmStorage();

        vm.resetGasMetering();
        tint.deposit(address(token), 1, bytes32(uint256(1)), "");
    }

    function test_toPublicInputs_gas() public {
        bytes32 startAggregationHash = bytes32(uint256(1));
        bytes32 endAggregationHash = bytes32(uint256(2));

        IPrivacyPool.Operation memory op;
        op.newRoot = bytes32(uint256(1));

        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.nullifiers[i] = bytes32(i + 1);
        }
        op.commitmentsOut[0] = bytes32(uint256(keccak256(abi.encode("commitment", uint256(0)))) % BN254_FR_MODULUS);
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        op.context.unshieldRecipients[0] = address(1);

        vm.resetGasMetering();
        tint.toPublicSignals(GENESIS_ROOT, startAggregationHash, endAggregationHash, op);
    }

    function test_verifyOperation_gas() public {
        tint.warmStorage();
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op;
        op.newRoot = bytes32(uint256(1));

        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.nullifiers[i] = bytes32(i + 1);
        }
        op.commitmentsOut[0] = bytes32(uint256(keccak256(abi.encode("commitment", uint256(0)))) % BN254_FR_MODULUS);
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        op.context.unshieldRecipients[0] = address(1);

        vm.resetGasMetering();
        tint.verifyOperation(op);
    }

    function test_operate_gas() public {
        tint.warmStorage();
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op;
        op.newRoot = bytes32(uint256(1));

        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.nullifiers[i] = bytes32(i + 1);
        }
        op.commitmentsOut[0] = bytes32(uint256(keccak256(abi.encode("commitment", uint256(0)))) % BN254_FR_MODULUS);
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        op.context.unshieldRecipients[0] = address(1);

        vm.resetGasMetering();
        tint.operate(op);
    }

    function test_operate_full_gas() public {
        tint.warmStorage();
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op;
        op.newRoot = bytes32(uint256(1));

        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.nullifiers[i] = bytes32(i + 1);
        }
        for (uint256 i = 0; i < N_OUTPUTS; i++) {
            op.commitmentsOut[i] = bytes32(i + 1);
        }
        for (uint120 i = 0; i < N_WITHDRAWALS; i++) {
            op.unshieldAmounts[i] = 1;
            op.unshieldAssets[i] = address(token);
            op.context.unshieldRecipients[i] = address(uint160(i + 1));
        }

        vm.resetGasMetering();
        tint.operate(op);
    }
}

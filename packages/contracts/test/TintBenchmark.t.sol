// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {TintVerifier} from "../src/TintVerifier.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ProofLib} from "../src/lib/ProofLib.sol";
import {N_PUB, N_COMPRESSED_PUB, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, BN254_FR_MODULUS} from "../src/lib/Constants.sol";

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

    function verify(uint256[8] calldata proof, uint256[N_COMPRESSED_PUB] calldata pubSignals) external view {
        try INNER.verify(proof, pubSignals) {} catch {}
    }
}

contract TintHarness is Tint {
    constructor(IVerifier _verifier) Tint(_verifier) {}

    function toPublicSignals(IPrivacyPool.Operation calldata op) external pure returns (uint256[N_PUB] memory) {
        return ProofLib.toPublicSignals(op);
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

        // Seed the MMR so benchmarks measure a steady-state frontier, not an empty one.
        for (uint256 i = 0; i < 64; i++) {
            tint.deposit(address(token), 1, bytes32(uint256(i + 1)), "");
        }
    }

    /// @dev An operation bound to the pool's live frontier, as `verifyOperation` requires.
    function _operation() internal view returns (IPrivacyPool.Operation memory op) {
        op.histState = tint.mmrState();
        op.frontier = tint.frontier();
    }

    function test_shield_gas() public {
        vm.resetGasMetering();
        tint.deposit(address(token), 1, bytes32(uint256(1)), "");
    }

    function test_toPublicInputs_gas() public {
        IPrivacyPool.Operation memory op = _operation();

        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.nullifiers[i] = bytes32(i + 1);
        }
        op.commitmentsOut[0] = bytes32(uint256(keccak256(abi.encode("commitment", uint256(0)))) % BN254_FR_MODULUS);
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        op.context.unshieldRecipients[0] = address(1);

        vm.resetGasMetering();
        tint.toPublicSignals(op);
    }

    function test_verifyOperation_gas() public {
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op = _operation();

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
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op = _operation();

        op.nullifiers[0] = bytes32(uint256(1));
        op.commitmentsOut[0] = bytes32(uint256(keccak256(abi.encode("commitment", uint256(0)))) % BN254_FR_MODULUS);
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        op.context.unshieldRecipients[0] = address(1);

        vm.resetGasMetering();
        tint.operate(op);
    }

    function test_operate_full_gas() public {
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op = _operation();

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

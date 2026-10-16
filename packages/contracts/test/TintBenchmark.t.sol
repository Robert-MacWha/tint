// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {IPoseidon2T3} from "../src/interfaces/IPoseidon2T3.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {TintVerifier} from "../src/TintVerifier.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ProofLib} from "../src/lib/ProofLib.sol";
import {OperationLib} from "./OperationLib.sol";
import {N_COMPRESSED_PUB, N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, BN254_FR_MODULUS} from "../src/lib/Constants.sol";

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
    using ProofLib for IPrivacyPool.Operation;
    constructor(IVerifier _verifier, IPoseidon2T3 _poseidon) Tint(_verifier, _poseidon) {}

    function toPublicSignals(IPrivacyPool.Operation calldata op) external pure returns (uint256[] memory) {
        return op.toPublicSignals();
    }

    function warmStorage() external {
        for (uint256 i = mmr.depth; i < mmr.maxDepth; i++) {
            mmr.roots[i] = bytes32(uint256(1));
            mmr.chains[i] = bytes32(uint256(1));
        }

        uint256 current = mmr.count % HISTORY_SIZE;
        for (uint256 i = 0; i < HISTORY_SIZE; i++) {
            if (i == current || history[i] != 0) continue;
            history[i] = bytes32(uint256(1));
        }
    }
}

contract TintGasReportTest is Test {
    using OperationLib for IPrivacyPool.Operation;

    TintHarness public tint;
    MockToken public token;

    function setUp() public {
        token = new MockToken();
        TintVerifier groth16Verifier = new TintVerifier();
        AlwaysTrueVerifier verifier = new AlwaysTrueVerifier(groth16Verifier);
        tint = new TintHarness(verifier, new Poseidon2T3());
        token.approve(address(tint), type(uint256).max);

        // Seed the MMR so benchmarks measure a steady-state frontier, not an empty one.
        for (uint256 i = 0; i < 64; i++) {
            tint.deposit(address(token), 1, bytes32(uint256(i + 1)), "");
        }
        tint.warmStorage();

        // Unshield recipients already hold the token, so their balances aren't zero -> non-zero writes.
        for (uint160 i = 0; i < N_WITHDRAWALS; i++) {
            require(token.transfer(address(i + 1), 1), "Transfer failed");
        }
    }

    /// @dev An operation bound to the pool's live frontier, as `verifyOperation` requires.
    function _operation() internal view returns (IPrivacyPool.Operation memory) {
        return OperationLib.bind(tint);
    }

    function test_shield_gas() public {
        vm.resetGasMetering();
        tint.deposit(address(token), 1, bytes32(uint256(1)), "");
    }

    function test_toPublicInputs_gas() public {
        IPrivacyPool.Operation memory op = _operation();

        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.setNullifier(i, bytes32(i + 1));
        }
        op.setCommitmentOut(0, bytes32(uint256(keccak256(abi.encode("commitment", uint256(0)))) % BN254_FR_MODULUS));
        op.setUnshieldAmount(0, 1);
        op.setUnshieldAsset(0, address(token));
        op.setUnshieldRecipient(0, address(1));

        vm.resetGasMetering();
        tint.toPublicSignals(op.seal());
    }

    function test_verifyOperation_gas() public {
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op = _operation();

        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.setNullifier(i, bytes32(i + 1));
        }
        op.setCommitmentOut(0, bytes32(uint256(keccak256(abi.encode("commitment", uint256(0)))) % BN254_FR_MODULUS));
        op.setUnshieldAmount(0, 1);
        op.setUnshieldAsset(0, address(token));
        op.setUnshieldRecipient(0, address(1));

        vm.resetGasMetering();
        tint.verifyOperation(op.seal());
    }

    function test_operate_gas() public {
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op = _operation();

        op.setNullifier(0, bytes32(uint256(1)));
        op.setCommitmentOut(0, bytes32(uint256(keccak256(abi.encode("commitment", uint256(0)))) % BN254_FR_MODULUS));
        op.setUnshieldAmount(0, 1);
        op.setUnshieldAsset(0, address(token));
        op.setUnshieldRecipient(0, address(1));

        vm.resetGasMetering();
        tint.operate(op.seal());
    }

    function test_operate_full_gas() public {
        require(token.transfer(address(tint), 1_000), "Transfer failed");

        IPrivacyPool.Operation memory op = _operation();

        for (uint256 i = 0; i < N_INPUTS; i++) {
            op.setNullifier(i, bytes32(i + 1));
        }
        for (uint256 i = 0; i < N_OUTPUTS; i++) {
            op.setCommitmentOut(i, bytes32(i + 1));
        }
        for (uint120 i = 0; i < N_WITHDRAWALS; i++) {
            op.setUnshieldAmount(i, 1);
            op.setUnshieldAsset(i, address(token));
            op.setUnshieldRecipient(i, address(uint160(i + 1)));
        }

        vm.resetGasMetering();
        tint.operate(op.seal());
    }
}

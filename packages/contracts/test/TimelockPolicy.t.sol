// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {Verifier} from "../src/codegen/TimelockVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ProofLib} from "../src/lib/ProofLib.sol";
import {FRONTIER_OFFSET, BN254_FR_MODULUS} from "../src/lib/Constants.sol";
import {TimelockPolicy} from "../src/policy/timelock/TimelockPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {OperationLib} from "./OperationLib.sol";

contract TimelockPolicyTest is Test {
    using OperationLib for IPrivacyPool.Operation;

    uint256 constant NOW = 1_000_000;
    uint256 constant INPUT = 1;

    Verifier verifier;
    Poseidon2T2 poseidon;
    TimelockPolicy policy;

    function setUp() public {
        vm.warp(NOW);
        verifier = new Verifier();
        poseidon = new Poseidon2T2();
        policy = new TimelockPolicy(verifier, poseidon);
    }

    function test_authorizesBoundAtTimestamp() public {
        IPrivacyPool.Operation memory op = _operation(NOW);
        _mockValidProof(op, NOW);

        policy.requireAuthorized(op);
    }

    function test_authorizesBoundBeforeTimestamp() public {
        IPrivacyPool.Operation memory op = _operation(NOW - 1);
        _mockValidProof(op, NOW - 1);

        policy.requireAuthorized(op);
    }

    function test_revertsBoundAfterTimestamp() public {
        IPrivacyPool.Operation memory op = _operation(NOW + 1);
        _mockValidProof(op, NOW + 1);

        vm.expectRevert(TimelockPolicy.Locked.selector);
        policy.requireAuthorized(op);
    }

    function test_revertsInvalidProof() public {
        IPrivacyPool.Operation memory op = _operation(NOW);

        vm.expectRevert(Verifier.ProofInvalid.selector);
        policy.requireAuthorized(op);
    }

    /// The proof binds the operation's signing hash, so it can't authorize another operation.
    function test_revertsOtherOperation() public {
        IPrivacyPool.Operation memory op = _operation(NOW);
        _mockValidProof(op, NOW);
        op.context.binding = bytes32(uint256(1));

        vm.expectRevert(Verifier.ProofInvalid.selector);
        policy.requireAuthorized(op);
    }

    /// Placeholder inputs must reach the verifier, whatever their bound, to spend a valid input's gas.
    function test_placeholderRevertsInVerifier() public {
        IPrivacyPool.Operation memory op = _operation(NOW + 1);

        vm.expectRevert(Verifier.ProofInvalid.selector);
        policy.requireAuthorized(op);
    }

    function test_revertsNoInputsForThisPolicy() public {
        IPrivacyPool.Operation memory op = _operation(NOW);
        op.setPolicyAddress(INPUT, address(0));

        vm.expectRevert(TimelockPolicy.NoInputsForThisPolicy.selector);
        policy.requireAuthorized(op);
    }

    /// An operation whose input `INPUT` is bound to `policy`, with a placeholder proof for `bound`.
    function _operation(uint256 bound) internal view returns (IPrivacyPool.Operation memory op) {
        op.pub = new bytes32[](FRONTIER_OFFSET);
        op.pub[1] = bytes32(uint256(1234));
        op.setPolicyAddress(INPUT, address(policy));

        ProofLib.Proof memory proof;
        op.context.policyInputs[INPUT] = abi.encode(proof, bound);
    }

    /// Makes the verifier accept `op`'s placeholder proof for exactly `op`'s signals.
    function _mockValidProof(IPrivacyPool.Operation memory op, uint256 bound) internal {
        uint256 contextHash = uint256(
            keccak256(abi.encode(op.context.caller, op.context.binding, op.context.ciphertexts))
        ) % BN254_FR_MODULUS;
        uint256 signingHash = poseidon.compress(uint256(op.pub[1]), contextHash, 0);

        ProofLib.Proof memory proof;
        uint256[3] memory pubSignals = [uint256(uint160(address(policy))), signingHash, bound];
        vm.mockCall(address(verifier), abi.encodeCall(Verifier.verifyProof, (proof.proof, pubSignals)), "");
    }
}

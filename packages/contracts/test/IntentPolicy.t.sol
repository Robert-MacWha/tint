// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {IntentPolicy} from "../src/policy/intent/IntentPolicy.sol";
import {Verifier} from "../src/codegen/IntentVerifier.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ProofLib} from "../src/lib/ProofLib.sol";
import {OperationLib} from "./OperationLib.sol";
import {FRONTIER_OFFSET} from "../src/lib/Constants.sol";

contract MockIntentVerifier {
    function verifyProof(uint256[8] calldata, uint256[4] calldata) external view {}
}

contract IntentPolicyTest is Test {
    using OperationLib for IPrivacyPool.Operation;
    using ProofLib for IPrivacyPool.Operation;

    MockIntentVerifier verifier;
    Poseidon2T2 poseidon;
    IntentPolicy policy;

    uint64 constant DEADLINE = 1_000;

    function setUp() public {
        verifier = new MockIntentVerifier();
        poseidon = new Poseidon2T2();
        policy = new IntentPolicy(Verifier(address(verifier)), poseidon);
        vm.warp(DEADLINE);
    }

    /// Should verify each bound input's proof against its own slot and deadline.
    function test_verifiesEachBoundInput() public {
        IPrivacyPool.Operation memory op = _operation();
        _bind(op, 0, 1, DEADLINE);
        op.setPolicyAddress(1, address(0xbeef));
        _bind(op, 2, 2, DEADLINE + 1);

        uint256 hash = this.signingHash(op);
        vm.expectCall(address(verifier), _verifyCall(1, hash, 0, DEADLINE));
        vm.expectCall(address(verifier), _verifyCall(2, hash, 2, DEADLINE + 1));
        policy.requireAuthorized(op);
    }

    /// Should reject a proof whose deadline has passed.
    function test_revertsAfterDeadline() public {
        IPrivacyPool.Operation memory op = _operation();
        _bind(op, 0, 1, DEADLINE - 1);

        vm.expectRevert(IntentPolicy.Expired.selector);
        policy.requireAuthorized(op);
    }

    /// Should bubble a failed verification.
    function test_revertsOnInvalidProof() public {
        IPrivacyPool.Operation memory op = _operation();
        _bind(op, 0, 1, DEADLINE);

        vm.mockCallRevert(address(verifier), abi.encodeWithSelector(Verifier.verifyProof.selector), "invalid");
        vm.expectRevert("invalid");
        policy.requireAuthorized(op);
    }

    /// Should reject an operation with no input bound to this policy.
    function test_revertsWithoutBoundInputs() public {
        IPrivacyPool.Operation memory op = _operation();

        vm.expectRevert(IntentPolicy.NoInputsForThisPolicy.selector);
        policy.requireAuthorized(op);
    }

    function signingHash(IPrivacyPool.Operation calldata op) external view returns (uint256) {
        return op.signingHash(poseidon);
    }

    function _operation() internal pure returns (IPrivacyPool.Operation memory op) {
        op.pub = new bytes32[](FRONTIER_OFFSET);
        op.pub[1] = bytes32(uint256(42));
    }

    /// Binds input `slot` to the policy, with a proof tagged `tag`.
    function _bind(IPrivacyPool.Operation memory op, uint256 slot, uint256 tag, uint64 deadline) internal view {
        op.setPolicyAddress(slot, address(policy));
        op.context.policyInputs[slot] = abi.encode(_proof(tag), deadline);
    }

    function _verifyCall(uint256 tag, uint256 hash, uint256 slot, uint64 deadline)
        internal
        view
        returns (bytes memory)
    {
        uint256[4] memory signals = [uint256(uint160(address(policy))), hash, slot, deadline];
        return abi.encodeCall(Verifier.verifyProof, (_proof(tag).proof, signals));
    }

    function _proof(uint256 tag) internal pure returns (ProofLib.Proof memory proof) {
        proof.proof[0] = tag;
    }
}

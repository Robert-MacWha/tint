// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {Verifier} from "../src/codegen/HardwareWalletVerifier.sol";
import {HardwareWalletPolicy} from "../src/policy/hardware-wallet/HardwareWalletPolicy.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {OperationLib} from "./OperationLib.sol";
import {FRONTIER_OFFSET, BN254_FR_MODULUS} from "../src/lib/Constants.sol";

contract HardwareWalletPolicyTests is Test {
    using OperationLib for IPrivacyPool.Operation;

    Verifier verifier;
    Poseidon2T2 poseidon;
    HardwareWalletPolicy policy;

    function setUp() public {
        verifier = new Verifier();
        poseidon = new Poseidon2T2();
        policy = new HardwareWalletPolicy(verifier, poseidon);
    }

    /// Should compute the EIP-712 digest of `TintOperation(bytes32 signingHash)`.
    function test_operationDigest() public view {
        bytes32 domainSeparator = keccak256(
            abi.encode(
                keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
                keccak256("Tint Hardware Wallet Policy"),
                keccak256("1"),
                block.chainid,
                address(policy)
            )
        );
        bytes32 structHash = keccak256(abi.encode(keccak256("TintOperation(bytes32 signingHash)"), uint256(42)));
        bytes32 expected = keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));

        assertEq(policy.operationDigest(42), expected);
    }

    /// Should verify the proof of the input bound to this policy against the
    /// policy address, signing hash and digest halves.
    function test_requireAuthorized_verifiesSignals() public {
        IPrivacyPool.Operation memory op = _operation();
        bytes memory proof = hex"1234";
        op.context.policyInputs[1] = proof;
        op.setPolicyAddress(1, address(policy));

        uint256 signingHash = _signingHash(op);
        uint256 digest = uint256(policy.operationDigest(signingHash));
        uint256[4] memory signals = [uint256(uint160(address(policy))), signingHash, digest >> 128, uint128(digest)];
        bytes memory call = abi.encodeCall(Verifier.verifyProof, (proof, signals));
        vm.mockCall(address(verifier), call, "");
        vm.expectCall(address(verifier), call);

        policy.requireAuthorized(op);
    }

    /// Should revert if the proof is invalid.
    function test_requireAuthorized_invalidProof_reverts() public {
        IPrivacyPool.Operation memory op = _operation();
        op.context.policyInputs[0] = new bytes(384);
        op.setPolicyAddress(0, address(policy));

        vm.expectRevert();
        policy.requireAuthorized(op);
    }

    /// Should revert if no input is bound to this policy.
    function test_requireAuthorized_noInputs_reverts() public {
        vm.expectRevert(HardwareWalletPolicy.NoInputsForThisPolicy.selector);
        policy.requireAuthorized(_operation());
    }

    function _operation() internal pure returns (IPrivacyPool.Operation memory op) {
        op.pub = new bytes32[](FRONTIER_OFFSET);
        op.pub[1] = bytes32(uint256(7));
        op.context.binding = bytes32(uint256(8));
    }

    /// @dev Mirrors `ProofLib.signingHash` for a memory operation.
    function _signingHash(IPrivacyPool.Operation memory op) internal view returns (uint256) {
        uint256 contextHash = uint256(
            keccak256(abi.encode(op.context.caller, op.context.binding, op.context.ciphertexts))
        ) % BN254_FR_MODULUS;
        return poseidon.compress(uint256(op.pub[1]), contextHash, 0);
    }
}

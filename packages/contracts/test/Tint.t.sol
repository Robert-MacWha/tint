// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {ERC20, IERC20Errors} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IPolicy} from "../src/interfaces/IPolicy.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {Tint} from "../src/Tint.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {NullifierRegistry} from "../src/NullifierRegistry.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ProofLib} from "../src/lib/ProofLib.sol";
import {OperationLib} from "./OperationLib.sol";
import {N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, N_COMPRESSED_PUB, BN254_FR_MODULUS} from "../src/lib/Constants.sol";

contract MockToken is ERC20 {
    constructor() ERC20("Mock", "MCK") {}

    function mint(address to, uint256 amount) public {
        super._mint(to, amount);
    }
}

contract MockVerifier is IVerifier {
    bool shouldPass = true;

    function setPass(bool v) external {
        shouldPass = v;
    }

    function verify(uint256[8] calldata, uint256[N_COMPRESSED_PUB] calldata) external view {
        if (!shouldPass) {
            revert("Invalid proof");
        }
    }
}

contract MockPolicy is IPolicy {
    bool shouldPass = true;

    error NotAuthorized();

    function setPass(bool v) external {
        shouldPass = v;
    }

    function requireAuthorized(IPrivacyPool.Operation calldata) external view {
        if (!shouldPass) {
            revert NotAuthorized();
        }
    }
}

contract TintTests is Test {
    using OperationLib for IPrivacyPool.Operation;

    MockToken token;
    MockVerifier verifier;
    MockPolicy policy;
    Tint tint;

    function setUp() public {
        token = new MockToken();
        verifier = new MockVerifier();
        policy = new MockPolicy();
        tint = new Tint(verifier, new Poseidon2T3());
        token.mint(address(this), type(uint128).max);
        token.approve(address(tint), type(uint256).max);

        tint.deposit(address(token), 1, bytes32(uint256(0xdeadbeef)), "");
    }

    /// @dev Binds the operation to the pool's live frontier, which is what
    /// `verifyOperation` authenticates against.
    function _operation() internal view returns (IPrivacyPool.Operation memory) {
        return OperationLib.bind(tint);
    }

    /// @dev Sets the simulation flag, as a state override would.
    function _simulate() internal {
        bytes32 slot = keccak256(abi.encode(uint256(keccak256("tint.simulation")) - 1)) & ~bytes32(uint256(0xff));
        vm.store(address(tint), slot, bytes32(uint256(1)));
    }

    /// -------------------- deposit() --------------------

    /// Should be able to deposit. Depositing should transfer the correct amount
    /// of tokens from the caller to Tint and emit a Deposited event.
    function test_deposit() public {
        uint256 tintBefore = token.balanceOf(address(tint));
        uint256 callerBefore = token.balanceOf(address(this));

        vm.expectEmit();
        bytes32 commitment = 0x2ee5225f16cda90e5c31a84c3ff505613050d79f01b022a6a629ee951a050715;
        emit Tint.Deposited(commitment, uint64(tint.count()), address(token), 100, "");
        tint.deposit(address(token), 100, bytes32(uint256(1)), "");

        assertEq(token.balanceOf(address(this)), callerBefore - 100);
        assertEq(token.balanceOf(address(tint)), tintBefore + 100);
    }

    /// Should revert if the caller has not approved Tint to spend their tokens.
    function test_depositInsufficientBalance_reverts() public {
        MockToken fresh = new MockToken();
        fresh.mint(address(this), 1000);

        vm.expectRevert(abi.encodeWithSelector(IERC20Errors.ERC20InsufficientAllowance.selector, address(tint), 0, 500));
        tint.deposit(address(fresh), 500, bytes32(uint256(42)), "");
    }

    /// -------------------- operate() --------------------

    /// Should be able to operate. Operating should nullify the provided nullifiers,
    /// stage the provided output commitments, unshield the provide amounts, and emit
    /// the appropriate events.
    function test_operate() public {
        token.mint(address(tint), 100_000);

        IPrivacyPool.Operation memory op = _operation();
        op.setNullifier(0, bytes32(uint256(123)));
        op.setPolicyAddress(1, address(policy));
        op.setNullifier(1, bytes32(uint256(456)));
        op.setPolicyAddress(1, address(policy));
        op.setCommitmentOut(0, bytes32(uint256(42)));
        op.context.ciphertexts[0] = bytes("Hello");
        op.setCommitmentOut(1, bytes32(uint256(43)));
        op.context.ciphertexts[1] = bytes("World!");
        op.setUnshieldAmount(0, 1);
        op.setUnshieldAsset(0, address(token));
        op.setUnshieldRecipient(0, address(0xdead));
        op.setUnshieldAmount(1, 2);
        op.setUnshieldAsset(1, address(token));
        op.setUnshieldRecipient(1, address(0xbeef));

        uint256 tintBefore = token.balanceOf(address(tint));
        uint256 deadBefore = token.balanceOf(address(0xdead));
        uint256 beefBefore = token.balanceOf(address(0xbeef));

        //? Require events
        for (uint256 i; i < N_INPUTS; ++i) {
            if (op.nullifier(i) == bytes32(0)) continue;
            vm.expectEmit();
            emit Tint.Nullified(op.nullifier(i));
        }

        // Only non-zero outputs are appended; zero is the padding value.
        uint64 leafIndex = uint64(tint.count());
        for (uint256 i; i < N_OUTPUTS; ++i) {
            if (op.commitmentOut(i) == bytes32(0)) continue;
            vm.expectEmit();
            emit Tint.Committed(op.commitmentOut(i), leafIndex++, op.context.ciphertexts[i]);
        }

        for (uint256 i; i < N_WITHDRAWALS; ++i) {
            if (op.unshieldAmount(i) == 0) continue;
            vm.expectEmit();
            emit Tint.Withdrawn(op.unshieldAsset(i), op.unshieldAmount(i), op.unshieldRecipient(i));
        }

        tint.operate(op.seal());

        //? Check balance updates
        assertEq(token.balanceOf(address(tint)), tintBefore - 3);
        assertEq(token.balanceOf(address(0xdead)), deadBefore + 1);
        assertEq(token.balanceOf(address(0xbeef)), beefBefore + 2);
    }

    /// Should revert if the proof is invalid
    function test_operateInvalidProof_reverts() public {
        verifier.setPass(false);
        IPrivacyPool.Operation memory op = _operation();
        vm.expectRevert(Tint.InvalidProof.selector);
        tint.operate(op.seal());
    }

    /// Should revert if any of the nullifiers have already been spent
    function test_operateSpentNullifier_reverts() public {
        IPrivacyPool.Operation memory op = _operation();
        op.setNullifier(0, bytes32(uint256(123)));
        tint.operate(op.seal());

        vm.expectRevert(abi.encodeWithSelector(NullifierRegistry.NullifierAlreadySpent.selector, bytes32(uint256(123))));
        tint.operate(op.seal());
    }

    /// Should revert if a public signal is not a canonical field element.
    /// Hybrid compression reduces each signal modulo the field, so `nf` and
    /// `nf + p` would satisfy the same proof, while the nullifier registry
    /// would treat them as different nullifiers and let the note be spent again.
    function test_operateNonCanonicalNullifier_reverts() public {
        IPrivacyPool.Operation memory op = _operation();
        op.setNullifier(0, bytes32(uint256(123)));
        tint.operate(op.seal());

        op.setNullifier(0, bytes32(uint256(123) + BN254_FR_MODULUS));
        vm.expectRevert();
        tint.operate(op.seal());
    }

    /// Should revert if `context` changes after `pub[0]` was bound to it.
    function test_operateUnboundContext_reverts() public {
        IPrivacyPool.Operation memory op = _operation().seal();
        op.context.ciphertexts[0] = bytes("tampered");

        vm.expectRevert(ProofLib.InvalidPublicSignals.selector);
        tint.operate(op);
    }

    /// Should revert if any of the policy contracts revert.
    function test_operatePolicyReverts_reverts() public {
        IPrivacyPool.Operation memory op = _operation();
        op.setNullifier(0, bytes32(uint256(123)));
        op.setPolicyAddress(0, address(policy));
        policy.setPass(false);

        vm.expectRevert(MockPolicy.NotAuthorized.selector);
        tint.operate(op.seal());
    }

    /// -------------------- simulation --------------------

    /// Should ignore an invalid proof while simulating.
    function test_operateSimulatingInvalidProof() public {
        _simulate();
        verifier.setPass(false);
        IPrivacyPool.Operation memory op = _operation();
        op.setNullifier(0, bytes32(uint256(123)));

        tint.operate(op.seal());
        assertTrue(tint.isSpent(bytes32(uint256(123))));
    }

    /// Should ignore a reverting policy while simulating.
    function test_operateSimulatingPolicyReverts() public {
        _simulate();
        IPrivacyPool.Operation memory op = _operation();
        op.setNullifier(0, bytes32(uint256(123)));
        op.setPolicyAddress(0, address(policy));
        policy.setPass(false);

        tint.operate(op.seal());
        assertTrue(tint.isSpent(bytes32(uint256(123))));
    }

    /// -------------------- executePreVerified --------------------

    /// Should be able to execute a pre-verified operation.
    function test_preVerifyAllowsExecution() public {
        bytes32 slot = keccak256("slot");

        IPrivacyPool.Operation memory op = _operation();
        op.setNullifier(0, bytes32(uint256(123)));
        op = op.seal();

        tint.preVerify(slot, op);
        tint.executePreVerified(slot, op);
    }

    /// Should revert because the nullifier was already spent, even though the operation was pre-verified.
    function test_doubleSpendPreverified_reverts() public {
        bytes32 slotA = keccak256("slotA");
        bytes32 slotB = keccak256("slotB");

        IPrivacyPool.Operation memory op = _operation();
        op.setNullifier(0, bytes32(uint256(123)));
        op = op.seal();

        tint.preVerify(slotA, op);
        tint.preVerify(slotB, op);
        tint.executePreVerified(slotA, op);
        vm.expectRevert(abi.encodeWithSelector(NullifierRegistry.NullifierAlreadySpent.selector, bytes32(uint256(123))));
        tint.executePreVerified(slotB, op);
    }

    /// -------------------- frontier --------------------

    /// Should accept the pool's live frontier.
    function test_operateLiveFrontier() public {
        for (uint256 i = 0; i < 10; ++i) {
            tint.deposit(address(token), 1, bytes32(uint256(i + 1)), "");
        }

        tint.verifyOperation(_operation().seal());
    }

    /// Should reject an operation whose frontier the pool never committed to.
    ///
    /// @dev Covers that `verifyOperation` authenticates the frontier at all. Which
    /// frontiers `verifyFrontier` itself accepts is the skew-mmr package's concern.
    function test_operateUnauthenticatedFrontier_reverts() public {
        for (uint256 i = 0; i < 10; ++i) {
            tint.deposit(address(token), 1, bytes32(uint256(i + 1)), "");
        }

        IPrivacyPool.Operation memory op = _operation();
        op.setFrontierRoot(0, bytes32(uint256(op.frontierRoot(0)) ^ 1));

        vm.expectRevert(Tint.InvalidFrontier.selector);
        tint.verifyOperation(op.seal());
    }

    /// Should append only the non-zero output commitments. Zero is the padding
    /// value for an unused output slot and must not enter the set.
    function test_operateAppendsOnlyNonZeroOutputs() public {
        IPrivacyPool.Operation memory op = _operation();
        op.setCommitmentOut(0, bytes32(uint256(42)));
        op.setCommitmentOut(3, bytes32(uint256(43)));

        uint256 countBefore = tint.count();
        tint.operate(op.seal());

        assertEq(tint.count(), countBefore + 2);
    }
}

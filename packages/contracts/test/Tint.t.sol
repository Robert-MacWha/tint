// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {
    ERC20,
    IERC20Errors
} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ISpendability} from "../src/interfaces/ISpendability.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {Tint} from "../src/Tint.sol";
import {LibCircularBuffer} from "../src/lib/LibCircularBuffer.sol";
import {NullifierRegistry} from "../src/NullifierRegistry.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {
    N_INPUTS,
    N_OUTPUTS,
    N_WITHDRAWALS,
    N_PUB,
    AGGREGATION_RING_SIZE
} from "../src/lib/Constants.sol";

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

    function verifyProof(
        uint256[2] calldata,
        uint256[2][2] calldata,
        uint256[2] calldata,
        uint256[N_PUB] memory
    ) external view returns (bool) {
        return shouldPass;
    }
}

contract MockSpendability is ISpendability {
    bool shouldPass = true;

    error NotSpendable();

    function setPass(bool v) external {
        shouldPass = v;
    }

    function requireSpendable(IPrivacyPool.Operation calldata) external view {
        if (!shouldPass) {
            revert NotSpendable();
        }
    }
}

contract TintTests is Test {
    MockToken token;
    MockVerifier verifier;
    MockSpendability spendability;
    Tint tint;

    function setUp() public {
        token = new MockToken();
        verifier = new MockVerifier();
        spendability = new MockSpendability();
        tint = new Tint(address(verifier), AGGREGATION_RING_SIZE);
        token.mint(address(this), type(uint128).max);
        token.approve(address(tint), type(uint256).max);

        tint.deposit(address(token), 1, bytes32(uint256(0xdeadbeef)), "");
    }

    function _operation()
        internal
        pure
        returns (IPrivacyPool.Operation memory op)
    {
        op.newRoot = bytes32(uint256(1));
        op.endAggregationIndex = 0;
    }

    /// -------------------- deposit() --------------------

    /// Should be able to deposit. Depositing should transfer the correct amount
    /// of tokens from the caller to Tint and emit a Deposited event.
    function test_deposit() public {
        uint256 tintBefore = token.balanceOf(address(tint));
        uint256 callerBefore = token.balanceOf(address(this));

        vm.expectEmit();
        bytes32 commitment = 0x2ee5225f16cda90e5c31a84c3ff505613050d79f01b022a6a629ee951a050715;
        emit Tint.Deposited(commitment, "");
        tint.deposit(address(token), 100, bytes32(uint256(1)), "");

        assertEq(token.balanceOf(address(this)), callerBefore - 100);
        assertEq(token.balanceOf(address(tint)), tintBefore + 100);
    }

    /// Should revert if the caller has not approved Tint to spend their tokens.
    function test_depositInsufficientBalance_reverts() public {
        MockToken fresh = new MockToken();
        fresh.mint(address(this), 1000);

        vm.expectRevert(
            abi.encodeWithSelector(
                IERC20Errors.ERC20InsufficientAllowance.selector,
                address(tint),
                0,
                500
            )
        );
        tint.deposit(address(fresh), 500, bytes32(uint256(42)), "");
    }

    /// Should revert if the aggregation ring is full
    function test_depositRingFull_reverts() public {
        for (uint128 i = 0; i < AGGREGATION_RING_SIZE - 1; ++i) {
            tint.deposit(address(token), 1, bytes32(uint256(i + 1)), "");
        }

        vm.expectRevert(
            abi.encodeWithSelector(
                LibCircularBuffer.CircularBufferFull.selector
            )
        );
        tint.deposit(address(token), 1, bytes32(uint256(42)), "");
    }

    /// -------------------- operate() --------------------

    /// Should be able to operate. Operating should nullify the provided nullifiers,
    /// stage the provided output commitments, unshield the provide amounts, and emit
    /// the appropriate events.
    function test_operate() public {
        token.mint(address(tint), 100_000);

        IPrivacyPool.Operation memory op = _operation();
        op.nullifiers[0] = bytes32(uint256(123));
        op.spendabilityAddresses[1] = address(spendability);
        op.nullifiers[1] = bytes32(uint256(456));
        op.spendabilityAddresses[1] = address(spendability);
        op.commitmentsOut[0] = bytes32(uint256(42));
        op.context.ciphertexts[0] = bytes("Hello");
        op.commitmentsOut[1] = bytes32(uint256(43));
        op.context.ciphertexts[1] = bytes("World!");
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        op.context.unshieldRecipients[0] = address(0xdead);
        op.unshieldAmounts[1] = 2;
        op.unshieldAssets[1] = address(token);
        op.context.unshieldRecipients[1] = address(0xbeef);

        uint256 tintBefore = token.balanceOf(address(tint));
        uint256 deadBefore = token.balanceOf(address(0xdead));
        uint256 beefBefore = token.balanceOf(address(0xbeef));

        //? Require events
        for (uint256 i; i < N_INPUTS; ++i) {
            vm.expectEmit();
            emit Tint.Nullified(op.nullifiers[i]);
        }

        for (uint256 i; i < N_OUTPUTS; ++i) {
            vm.expectEmit();
            emit Tint.Committed(
                op.commitmentsOut[i],
                op.context.ciphertexts[i]
            );
        }

        for (uint256 i; i < N_WITHDRAWALS; ++i) {
            vm.expectEmit();
            emit Tint.Withdrawn(
                op.unshieldAssets[i],
                op.unshieldAmounts[i],
                op.context.unshieldRecipients[i]
            );
        }

        tint.operate(op);

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
        tint.operate(op);
    }

    /// Should revert if any of the nullifiers have already been spent
    function test_operateSpentNullifier_reverts() public {
        IPrivacyPool.Operation memory op = _operation();
        op.nullifiers[0] = bytes32(uint256(123));
        tint.operate(op);

        vm.expectRevert(
            abi.encodeWithSelector(
                NullifierRegistry.NullifierAlreadySpent.selector,
                bytes32(uint256(123))
            )
        );
        tint.operate(op);
    }

    /// Should revert if any of the spendability contracts revert.
    function test_operateSpendabilityReverts_reverts() public {
        IPrivacyPool.Operation memory op = _operation();
        op.nullifiers[0] = bytes32(uint256(123));
        op.spendabilityAddresses[0] = address(spendability);
        spendability.setPass(false);

        vm.expectRevert(MockSpendability.NotSpendable.selector);
        tint.operate(op);
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {EntryPoint} from "@account-abstraction/contracts/core/EntryPoint.sol";
import {IPaymaster} from "@account-abstraction/contracts/interfaces/IPaymaster.sol";
import {PackedUserOperation} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";
import {Tint} from "../src/Tint.sol";
import {TintPaymaster} from "../src/TintPaymaster.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {OperationLib} from "./OperationLib.sol";
import {MockToken, MockVerifier} from "./Tint.t.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

contract TintPaymasterTests is Test {
    using OperationLib for IPrivacyPool.Operation;

    uint256 constant MAX_COST = 0.5 ether;
    uint256 constant GAS_COST = 0.1 ether;
    uint256 constant FEE_PER_GAS = 1 gwei;
    address constant SENDER = address(0x5e7d);
    address constant RECIPIENT = address(0xbeef);
    address constant REFUND_RECIPIENT = address(0xfeed);

    EntryPoint entryPoint;
    MockToken weth;
    Tint tint;
    TintPaymaster paymaster;

    function setUp() public {
        entryPoint = new EntryPoint();
        weth = new MockToken();
        tint = new Tint(new MockVerifier(), new Poseidon2T3());
        paymaster = new TintPaymaster(entryPoint, tint, weth);

        weth.mint(address(this), 1);
        weth.approve(address(tint), 1);
        tint.deposit(address(weth), 1, bytes32(uint256(0xdeadbeef)), "");
        weth.mint(address(tint), 10 ether);
    }

    /// Should execute the operation, paying the fee to the paymaster and the
    /// rest to the recipient.
    function test_validate() public {
        IPrivacyPool.Operation memory op = _operation(address(weth), MAX_COST, _noRefund());
        bytes32 nullifier = op.nullifier(0);

        _validate(SENDER, op, _noRefund());

        assertEq(weth.balanceOf(address(paymaster)), MAX_COST);
        assertEq(weth.balanceOf(RECIPIENT), 2 ether);
        assertTrue(tint.isSpent(nullifier));
    }

    /// Should revert if the operation is bound to a different sender.
    function test_validateWrongBinding_reverts() public {
        IPrivacyPool.Operation memory op = _operation(address(weth), MAX_COST, _noRefund());

        vm.expectRevert(TintPaymaster.InvalidBinding.selector);
        _validate(address(0xbad), op, _noRefund());
    }

    /// Should revert if the refund differs from the one the operation is bound to.
    function test_validateWrongRefund_reverts() public {
        IPrivacyPool.Operation memory op = _operation(address(weth), MAX_COST, _transferRefund());

        TintPaymaster.Refund memory refund = _transferRefund();
        refund.recipient = address(0xbad);
        vm.expectRevert(TintPaymaster.InvalidBinding.selector);
        _validate(SENDER, op, refund);
    }

    /// Should revert if the fee doesn't cover `maxCost`.
    function test_validateInsufficientFee_reverts() public {
        IPrivacyPool.Operation memory op = _operation(address(weth), MAX_COST - 1, _noRefund());

        vm.expectRevert(abi.encodeWithSelector(TintPaymaster.InsufficientFee.selector, MAX_COST, MAX_COST - 1));
        _validate(SENDER, op, _noRefund());
    }

    /// Should not count fees paid in assets other than WETH.
    function test_validateNonWethFee_reverts() public {
        MockToken other = new MockToken();
        IPrivacyPool.Operation memory op = _operation(address(other), MAX_COST, _noRefund());

        vm.expectRevert(abi.encodeWithSelector(TintPaymaster.InsufficientFee.selector, MAX_COST, 0));
        _validate(SENDER, op, _noRefund());
    }

    /// Should revert on paymaster data that isn't an encoded operation.
    function test_validateMalformedData_reverts() public {
        PackedUserOperation memory userOp;
        userOp.sender = SENDER;
        userOp.paymasterAndData = abi.encodePacked(address(paymaster), uint128(0), uint128(0), hex"deadbeef");

        vm.expectRevert();
        vm.prank(address(entryPoint));
        paymaster.validatePaymasterUserOp(userOp, bytes32(0), MAX_COST);
    }

    /// Should only be callable by the EntryPoint.
    function test_validateNotEntryPoint_reverts() public {
        PackedUserOperation memory userOp =
            _userOp(SENDER, _operation(address(weth), MAX_COST, _noRefund()), _noRefund());

        vm.expectRevert("Sender not EntryPoint");
        paymaster.validatePaymasterUserOp(userOp, bytes32(0), MAX_COST);
    }

    /// Should only let the paymaster execute refunds.
    function test_executeRefundNotSelf_reverts() public {
        vm.expectRevert(TintPaymaster.OnlySelf.selector);
        paymaster.executeRefund(1, _transferRefund());
    }

    /// Should skip `postOp` when there's no refund.
    function test_validateNoRefund_emptyContext() public {
        bytes memory context = _validate(SENDER, _operation(address(weth), MAX_COST, _noRefund()), _noRefund());
        assertEq(context.length, 0);
    }

    /// Should transfer the fee left after gas and the buffer to the refund recipient.
    function test_postOpTransferRefund() public {
        bytes memory context =
            _validate(SENDER, _operation(address(weth), MAX_COST, _transferRefund()), _transferRefund());

        _postOp(context, GAS_COST);

        uint256 refund = MAX_COST - GAS_COST - paymaster.REFUND_GAS_BUFFER() * FEE_PER_GAS;
        assertEq(weth.balanceOf(REFUND_RECIPIENT), refund);
        assertEq(weth.balanceOf(address(paymaster)), MAX_COST - refund);
    }

    /// Should deposit the fee left after gas and the buffer into Tint.
    function test_postOpDepositRefund() public {
        bytes memory context =
            _validate(SENDER, _operation(address(weth), MAX_COST, _depositRefund()), _depositRefund());
        uint256 count = tint.count();
        uint256 tintBalance = weth.balanceOf(address(tint));

        _postOp(context, GAS_COST);

        uint256 refund = MAX_COST - GAS_COST - paymaster.REFUND_GAS_BUFFER() * FEE_PER_GAS;
        assertEq(tint.count(), count + 1);
        assertEq(weth.balanceOf(address(tint)), tintBalance + refund);
        assertEq(weth.balanceOf(address(paymaster)), MAX_COST - refund);
    }

    /// Should not refund if gas and the buffer use up the fee.
    function test_postOpFeeExhausted_noRefund() public {
        bytes memory context =
            _validate(SENDER, _operation(address(weth), MAX_COST, _transferRefund()), _transferRefund());

        _postOp(context, MAX_COST - paymaster.REFUND_GAS_BUFFER() * FEE_PER_GAS);

        assertEq(weth.balanceOf(REFUND_RECIPIENT), 0);
        assertEq(weth.balanceOf(address(paymaster)), MAX_COST);
    }

    /// Should keep the fee if the refund fails.
    function test_postOpRefundFails_keepsFee() public {
        bytes memory context =
            _validate(SENDER, _operation(address(weth), MAX_COST, _depositRefund()), _depositRefund());
        vm.mockCallRevert(address(tint), abi.encodeWithSelector(Tint.deposit.selector), "");

        _postOp(context, GAS_COST);

        assertEq(weth.balanceOf(address(paymaster)), MAX_COST);
    }

    /// Should let the owner sweep collected WETH.
    function test_sweep() public {
        _validate(SENDER, _operation(address(weth), MAX_COST, _noRefund()), _noRefund());

        paymaster.sweep(RECIPIENT, MAX_COST);
        assertEq(weth.balanceOf(RECIPIENT), 2 ether + MAX_COST);
    }

    /// Should revert if a non-owner tries to sweep.
    function test_sweepNotOwner_reverts() public {
        _validate(SENDER, _operation(address(weth), MAX_COST, _noRefund()), _noRefund());

        vm.prank(address(0xbad));
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, address(0xbad)));
        paymaster.sweep(RECIPIENT, MAX_COST);
    }

    /// @dev An operation bound to `SENDER` and `refund` that spends one note,
    /// unshields `fee` of `feeAsset` to the paymaster and 2 WETH to `RECIPIENT`.
    function _operation(address feeAsset, uint256 fee, TintPaymaster.Refund memory refund)
        internal
        view
        returns (IPrivacyPool.Operation memory op)
    {
        op = OperationLib.bind(tint);
        op.setNullifier(0, bytes32(uint256(123)));
        op.setUnshieldAsset(0, feeAsset);
        op.setUnshieldAmount(0, uint128(fee));
        op.setUnshieldRecipient(0, address(paymaster));
        op.setUnshieldAsset(1, address(weth));
        op.setUnshieldAmount(1, 2 ether);
        op.setUnshieldRecipient(1, RECIPIENT);
        op.context.binding = paymaster.binding(SENDER, refund);
        op.seal();
    }

    function _noRefund() internal pure returns (TintPaymaster.Refund memory refund) {}

    function _transferRefund() internal pure returns (TintPaymaster.Refund memory refund) {
        refund.kind = TintPaymaster.RefundKind.Transfer;
        refund.recipient = REFUND_RECIPIENT;
    }

    function _depositRefund() internal pure returns (TintPaymaster.Refund memory refund) {
        refund.kind = TintPaymaster.RefundKind.Deposit;
        refund.partialCommitment = bytes32(uint256(0xcafe));
        refund.encryptedPartial = hex"c0ffee";
    }

    function _validate(address sender, IPrivacyPool.Operation memory op, TintPaymaster.Refund memory refund)
        internal
        returns (bytes memory context)
    {
        vm.prank(address(entryPoint));
        (context,) = paymaster.validatePaymasterUserOp(_userOp(sender, op, refund), bytes32(0), MAX_COST);
    }

    function _postOp(bytes memory context, uint256 actualGasCost) internal {
        vm.prank(address(entryPoint));
        paymaster.postOp(IPaymaster.PostOpMode.opSucceeded, context, actualGasCost, FEE_PER_GAS);
    }

    function _userOp(address sender, IPrivacyPool.Operation memory op, TintPaymaster.Refund memory refund)
        internal
        view
        returns (PackedUserOperation memory userOp)
    {
        userOp.sender = sender;
        userOp.paymasterAndData = abi.encodePacked(address(paymaster), uint128(0), uint128(0), abi.encode(op, refund));
    }
}

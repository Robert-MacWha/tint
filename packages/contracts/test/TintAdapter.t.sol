// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ReentrancyGuardTransient} from "@openzeppelin/contracts/utils/ReentrancyGuardTransient.sol";
import {Tint} from "../src/Tint.sol";
import {TintAdapter} from "../src/TintAdapter.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {OperationLib} from "./OperationLib.sol";
import {MockToken, MockVerifier} from "./Tint.t.sol";

/// Swaps at a fixed rate, minting the output.
contract MockDapp {
    error Failed();

    function swap(IERC20 tokenIn, uint256 amountIn, MockToken tokenOut, uint256 amountOut) external {
        tokenIn.transferFrom(msg.sender, address(this), amountIn);
        tokenOut.mint(msg.sender, amountOut);
    }

    function fail() external pure {
        revert Failed();
    }

    function reenter(
        TintAdapter adapter,
        IPrivacyPool.Operation calldata op,
        TintAdapter.Call[] calldata calls,
        TintAdapter.Shield[] calldata shields
    ) external {
        adapter.relay(op, calls, shields);
    }
}

contract TintAdapterTests is Test {
    using OperationLib for IPrivacyPool.Operation;

    uint128 constant AMOUNT_IN = 2 ether;
    uint256 constant AMOUNT_OUT = 3 ether;
    bytes32 constant PARTIAL = bytes32(uint256(0xcafe));

    MockToken weth;
    MockToken usdc;
    Tint tint;
    TintAdapter adapter;
    MockDapp dapp;

    function setUp() public {
        weth = new MockToken();
        usdc = new MockToken();
        tint = new Tint(new MockVerifier(), new Poseidon2T3());
        adapter = new TintAdapter(tint);
        dapp = new MockDapp();

        weth.mint(address(this), 1);
        weth.approve(address(tint), 1);
        tint.deposit(address(weth), 1, bytes32(uint256(0xdeadbeef)), "");
        weth.mint(address(tint), 10 ether);
    }

    /// Should execute the operation, make the calls and shield the output.
    function test_relay() public {
        TintAdapter.Call[] memory calls = _swapCalls(AMOUNT_IN);
        TintAdapter.Shield[] memory shields = _shields(address(usdc));
        IPrivacyPool.Operation memory op = _operation(calls, shields);
        uint256 count = tint.count();

        _expectShield(address(usdc), AMOUNT_OUT, count);
        adapter.relay(op, calls, shields);

        assertTrue(tint.isSpent(op.nullifier(0)));
        assertEq(tint.count(), count + 1);
        assertEq(usdc.balanceOf(address(tint)), AMOUNT_OUT);
        assertEq(usdc.balanceOf(address(adapter)), 0);
        assertEq(weth.balanceOf(address(adapter)), 0);
        assertEq(usdc.allowance(address(adapter), address(tint)), 0);
    }

    /// Should shield whatever the calls leave of the unshielded input.
    function test_relayShieldsLeftover() public {
        TintAdapter.Call[] memory calls = _swapCalls(AMOUNT_IN - 1 ether);
        TintAdapter.Shield[] memory shields = new TintAdapter.Shield[](2);
        shields[0] = _shield(address(usdc));
        shields[1] = _shield(address(weth));
        IPrivacyPool.Operation memory op = _operation(calls, shields);
        uint256 count = tint.count();

        _expectShield(address(usdc), AMOUNT_OUT, count);
        _expectShield(address(weth), 1 ether, count + 1);
        adapter.relay(op, calls, shields);

        assertEq(tint.count(), count + 2);
        assertEq(weth.balanceOf(address(adapter)), 0);
    }

    /// Should skip shielding an asset the adapter holds none of.
    function test_relayEmptyShield_skipped() public {
        TintAdapter.Call[] memory calls = _swapCalls(AMOUNT_IN);
        TintAdapter.Shield[] memory shields = new TintAdapter.Shield[](2);
        shields[0] = _shield(address(usdc));
        shields[1] = _shield(address(weth));
        IPrivacyPool.Operation memory op = _operation(calls, shields);
        uint256 count = tint.count();

        adapter.relay(op, calls, shields);

        assertEq(tint.count(), count + 1);
    }

    /// Should revert if the calls differ from the ones the operation is bound to.
    function test_relayWrongCalls_reverts() public {
        TintAdapter.Shield[] memory shields = _shields(address(usdc));
        IPrivacyPool.Operation memory op = _operation(_swapCalls(AMOUNT_IN), shields);

        vm.expectRevert(TintAdapter.InvalidBinding.selector);
        adapter.relay(op, _swapCalls(AMOUNT_IN - 1), shields);
    }

    /// Should revert if the shields differ from the ones the operation is bound to.
    function test_relayWrongShields_reverts() public {
        TintAdapter.Call[] memory calls = _swapCalls(AMOUNT_IN);
        IPrivacyPool.Operation memory op = _operation(calls, _shields(address(usdc)));

        TintAdapter.Shield[] memory shields = _shields(address(usdc));
        shields[0].partialCommitment = bytes32(uint256(0xbad));
        vm.expectRevert(TintAdapter.InvalidBinding.selector);
        adapter.relay(op, calls, shields);
    }

    /// Should revert, leaving the notes unspent, if a call fails.
    function test_relayCallFails_reverts() public {
        TintAdapter.Call[] memory calls = new TintAdapter.Call[](1);
        calls[0] = TintAdapter.Call(address(dapp), 0, abi.encodeCall(MockDapp.fail, ()));
        TintAdapter.Shield[] memory shields = _shields(address(weth));
        IPrivacyPool.Operation memory op = _operation(calls, shields);

        vm.expectRevert(MockDapp.Failed.selector);
        adapter.relay(op, calls, shields);
        assertFalse(tint.isSpent(op.nullifier(0)));
    }

    /// Should revert on a call to Tint, which could execute an unbound operation.
    function test_relayCallToTint_reverts() public {
        TintAdapter.Call[] memory calls = new TintAdapter.Call[](1);
        calls[0] = TintAdapter.Call(address(tint), 0, "");
        TintAdapter.Shield[] memory shields = _shields(address(weth));
        IPrivacyPool.Operation memory op = _operation(calls, shields);

        vm.expectRevert(TintAdapter.CallToTint.selector);
        adapter.relay(op, calls, shields);
    }

    /// Should revert if a call re-enters `relay`.
    function test_relayReentrant_reverts() public {
        TintAdapter.Call[] memory noCalls;
        TintAdapter.Shield[] memory innerShields = _shields(address(weth));
        IPrivacyPool.Operation memory inner = _operation(noCalls, innerShields);

        TintAdapter.Call[] memory calls = new TintAdapter.Call[](1);
        calls[0] = TintAdapter.Call(
            address(dapp), 0, abi.encodeCall(MockDapp.reenter, (adapter, inner, noCalls, innerShields))
        );
        TintAdapter.Shield[] memory shields = _shields(address(weth));
        IPrivacyPool.Operation memory op = _operation(calls, shields);
        op.setNullifier(0, bytes32(uint256(456)));

        vm.expectRevert(ReentrancyGuardTransient.ReentrancyGuardReentrantCall.selector);
        adapter.relay(op, calls, shields);
    }

    /// @dev An operation bound to the adapter, `calls` and `shields` that spends
    /// one note and unshields `AMOUNT_IN` WETH to the adapter.
    function _operation(TintAdapter.Call[] memory calls, TintAdapter.Shield[] memory shields)
        internal
        view
        returns (IPrivacyPool.Operation memory op)
    {
        op = OperationLib.bind(tint);
        op.setNullifier(0, bytes32(uint256(123)));
        op.setUnshieldAsset(0, address(weth));
        op.setUnshieldAmount(0, AMOUNT_IN);
        op.setUnshieldRecipient(0, address(adapter));
        op.context.caller = address(adapter);
        op.context.binding = adapter.binding(calls, shields);
        op.seal();
    }

    /// @dev Approves and swaps `amountIn` WETH for `AMOUNT_OUT` USDC.
    function _swapCalls(uint256 amountIn) internal view returns (TintAdapter.Call[] memory calls) {
        calls = new TintAdapter.Call[](2);
        calls[0] = TintAdapter.Call(address(weth), 0, abi.encodeCall(IERC20.approve, (address(dapp), amountIn)));
        calls[1] = TintAdapter.Call(address(dapp), 0, abi.encodeCall(MockDapp.swap, (weth, amountIn, usdc, AMOUNT_OUT)));
    }

    /// @dev Expects a deposit of `amount` of `asset` for `PARTIAL` at `leafIndex`.
    function _expectShield(address asset, uint256 amount, uint256 leafIndex) internal {
        bytes32 commitment = bytes32(tint.POSEIDON().compress(uint256(uint160(asset)), amount, uint256(PARTIAL), 0));
        vm.expectEmit(address(tint));
        emit Tint.Deposited(commitment, uint64(leafIndex), asset, uint128(amount), hex"c0ffee");
    }

    function _shields(address asset) internal pure returns (TintAdapter.Shield[] memory shields) {
        shields = new TintAdapter.Shield[](1);
        shields[0] = _shield(asset);
    }

    function _shield(address asset) internal pure returns (TintAdapter.Shield memory) {
        return TintAdapter.Shield(asset, PARTIAL, hex"c0ffee");
    }
}

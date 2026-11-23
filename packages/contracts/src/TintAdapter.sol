// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SafeERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {SafeCast} from "@openzeppelin/contracts/utils/math/SafeCast.sol";
import {Address} from "@openzeppelin/contracts/utils/Address.sol";
import {ReentrancyGuardTransient} from "@openzeppelin/contracts/utils/ReentrancyGuardTransient.sol";

import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";

/// @notice Lets a Tint operation interact with dapps atomically. Relays an
/// operation that unshields to this adapter, makes its bound calls, then shields
/// the adapter's resulting balances back into Tint as new notes.
///
/// @dev The operation must name this adapter as its `context.caller` (which Tint
/// enforces) and carry `binding(calls, shields)`, so whoever submits the relay
/// can't alter its calls or shields. Any failing call reverts the whole relay,
/// operation included.
///
/// The adapter holds nothing between relays: any balance it's left with goes to
/// the next relay that shields that asset. Shield every asset a relay may leave
/// behind, including leftovers of the unshielded inputs.
///
/// Calls run as the adapter, so allowances they grant persist across relays.
/// Approve exactly what the dapp pulls, or reset it with a trailing call.
///
/// Minimum outputs are the dapp's concern (e.g. Uniswap's `amountOutMinimum`).
///
/// Native ETH outputs aren't shielded. Calls may unwrap WETH to spend ETH, but
/// must wrap any ETH they receive back themselves.
contract TintAdapter is ReentrancyGuardTransient {
    using SafeERC20 for IERC20;

    IPrivacyPool public immutable TINT;

    /// A call the adapter makes after executing the operation.
    struct Call {
        address target;
        uint256 value;
        bytes data;
    }

    /// Shields the adapter's whole balance of `asset` as a note for `partialCommitment`.
    struct Shield {
        address asset;
        bytes32 partialCommitment;
        bytes encryptedPartial;
    }

    error InvalidBinding();
    error CallToTint();

    constructor(IPrivacyPool _tint) {
        TINT = _tint;
    }

    /// Receives ETH unwrapped from WETH by a call.
    receive() external payable {}

    /// Executes `op`, makes `calls`, then shields the resulting balances per `shields`.
    function relay(IPrivacyPool.Operation calldata op, Call[] calldata calls, Shield[] calldata shields)
        external
        nonReentrant
    {
        if (op.context.binding != binding(calls, shields)) revert InvalidBinding();

        TINT.operate(op);
        for (uint256 i; i < calls.length; ++i) {
            //? Tint lets this adapter execute any operation naming it as caller, so a
            //? call to Tint could execute another operation without its binding.
            if (calls[i].target == address(TINT)) revert CallToTint();
            // forge-lint: disable-next-line(arbitrary-send-eth)
            Address.functionCallWithValue(calls[i].target, calls[i].data, calls[i].value);
        }
        for (uint256 i; i < shields.length; ++i) {
            _shield(shields[i]);
        }
    }

    /// The binding an operation must carry to be relayed with `calls` and `shields`.
    function binding(Call[] calldata calls, Shield[] calldata shields) public pure returns (bytes32) {
        return keccak256(abi.encode(calls, shields));
    }

    /// Deposits the adapter's whole balance of `shield.asset` into Tint.
    function _shield(Shield calldata shield) internal {
        IERC20 asset = IERC20(shield.asset);
        uint256 amount = asset.balanceOf(address(this));
        if (amount == 0) return;

        asset.forceApprove(address(TINT), amount);
        TINT.deposit(shield.asset, SafeCast.toUint128(amount), shield.partialCommitment, shield.encryptedPartial);
    }
}

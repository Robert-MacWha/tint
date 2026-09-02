// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SafeERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {BasePaymaster} from "@account-abstraction/contracts/core/BasePaymaster.sol";
import {UserOperationLib} from "@account-abstraction/contracts/core/UserOperationLib.sol";
import {IEntryPoint} from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import {PackedUserOperation} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";

import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";

/// @notice EIP-7562-compatible paymaster contract for Tint, designed to work with
/// the v0.8.0 EntryPoint contract.
///
/// @dev This paymaster intentionally only supports WETH as its fee token. This can be
/// changed if needed using a TWAP oracle.
///
/// @dev This paymaster is vulnerable to front-running attacks. If a user
/// invalidates their operation between `validatePaymasterUserOp` and `postOp`
/// (for example by nullifying the utxo in `executeUserOp`) the paymaster will
/// be unable to recover its fee.
///
/// This can be prevented by either:
///  1. Using the v0.6.0 EntryPoint, which allows the paymaster to revert in `postOp`
///     which unwinds the userOp and calls `postOp` again.
///  2. Enforcing a particular sender impl that always immediately executes the operation.
///  3. Using a 8141-style paymaster. This way different calls are represented by
///     frames. The validation frame can assert that the subsequent frame MUST
///     be a call to `POOL.executePreVerified` with the same slot & operation.
contract TintPaymaster is BasePaymaster {
    using UserOperationLib for PackedUserOperation;
    using SafeERC20 for IERC20;

    struct PaymasterData {
        /// Sender of the operation. MUST equal `userOp.sender`.
        address sender;
        /// Operation being sponsored.
        ///
        /// @dev The first withdrawal in the operation MUST pay the paymaster's fee.
        IPrivacyPool.Operation operation;

        /// Partial commitment hash for the refund target.
        bytes32 refundPartialCommitment;
        /// Encrypted partial commitment for the refund target.
        bytes refundPartialEncrypted;
    }

    IERC20 public immutable WETH;
    IPrivacyPool public immutable POOL;
    /// @dev The amount of gas required to execute postOp. Actual gas may be lower.
    uint128 public constant POST_OP_GAS_COST = 100_000;
    uint128 public constant MIN_REFUND = 10_000 gwei;

    constructor(IEntryPoint _entryPoint, IPrivacyPool _pool, IERC20 _weth) BasePaymaster(_entryPoint) {
        WETH = _weth;
        POOL = _pool;

        WETH.forceApprove(address(POOL), type(uint256).max);
    }

    // -------------------- BasePaymaster overrides --------------------
    function _validatePaymasterUserOp(PackedUserOperation calldata userOp, bytes32, uint256 maxCost)
        internal
        virtual
        override
        returns (bytes memory context, uint256 validationData)
    {
        bytes calldata paymasterData = _unpackPaymasterData(userOp.paymasterAndData);
        PaymasterData memory data = abi.decode(paymasterData, (PaymasterData));

        require(data.sender == userOp.sender, "sender mismatch");
        _requireSufficientPostOpGas(userOp);
        _requireFeePayment(data, maxCost);

        bytes32 associatedSlot = _getAssociatedSlot(data);
        POOL.preVerify(associatedSlot, data.operation);

        return (paymasterData, 0);
    }

    function _postOp(PostOpMode, bytes calldata context, uint256 actualGasCost, uint256 actualUserOpFeePerGas)
        internal
        virtual
        override
    {
        PaymasterData memory data = abi.decode(context, (PaymasterData));

        bytes32 associatedSlot = _getAssociatedSlot(data);
        POOL.executePreVerified(associatedSlot, data.operation);

        // forge-lint: disable-next-line(unsafe-typecast)
        _refund(data, uint128(actualGasCost), uint128(actualUserOpFeePerGas));
    }

    function _unpackPaymasterData(bytes calldata paymasterAndData) internal pure returns (bytes calldata data) {
        require(paymasterAndData.length >= UserOperationLib.PAYMASTER_DATA_OFFSET, "payamsterAndData too short");
        return paymasterAndData[UserOperationLib.PAYMASTER_DATA_OFFSET:];
    }

    /// EIP-7562 allows staked entities to write "Associated Storage" slots,
    /// where associated slots include the sender address and various hashes
    /// of the address.
    ///
    /// https://eips.ethereum.org/EIPS/eip-7562#definitions
    function _getAssociatedSlot(PaymasterData memory data) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(data.sender)));
    }

    function _requireSufficientPostOpGas(PackedUserOperation calldata userOp) internal pure {
        uint256 postOpGasLimit = userOp.unpackPostOpGasLimit();
        require(postOpGasLimit >= POST_OP_GAS_COST, "postOp gas limit too low");
    }

    /// Requires that the operation pays at least `requiredFee` in WETH to the paymaster.
    function _requireFeePayment(PaymasterData memory data, uint256 requiredFee) internal view {
        if (_feePaid(data) < requiredFee) {
            revert("insufficient fee paid");
        }
    }

    /// Refunds the sender the difference between the fee paid and the actual fee incurred by the
    /// paymaster.
    ///
    /// @dev Refunds the payer as a deposit into the privacy pool which can be withdrawn later.
    /// @dev Refunds are skipped if no refund commitment is provided, or if the refund is below the minimum threshold.
    function _refund(PaymasterData memory data, uint128 actualGasCost, uint128 actualUserOpFeePerGas) internal {
        uint128 totalFeePaid = _feePaid(data);

        uint128 actualFee = actualGasCost + POST_OP_GAS_COST * actualUserOpFeePerGas;
        uint128 refund = totalFeePaid - actualFee;

        if (refund < MIN_REFUND) {
            return;
        }

        if (data.refundPartialCommitment == bytes32(0)) {
            return;
        }

        POOL.deposit(address(WETH), refund, data.refundPartialCommitment, data.refundPartialEncrypted);
    }

    function _feePaid(PaymasterData memory data) internal view returns (uint128) {
        uint128 feePaid = 0;
        for (uint256 i = 0; i < data.operation.unshieldAmounts.length; i++) {
            if (data.operation.unshieldAssets[i] != address(WETH)) {
                continue;
            }

            if (data.operation.context.unshieldRecipients[i] != address(this)) {
                continue;
            }

            feePaid += data.operation.unshieldAmounts[i];
        }

        return feePaid;
    }
}

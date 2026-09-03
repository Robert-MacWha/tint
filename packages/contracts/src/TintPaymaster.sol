// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SafeERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {BasePaymaster} from "@account-abstraction/contracts/core/BasePaymaster.sol";
import {UserOperationLib} from "@account-abstraction/contracts/core/UserOperationLib.sol";
import {IEntryPoint} from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import {PackedUserOperation} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";

import {Tint} from "./Tint.sol";
import {IVerifier} from "./interfaces/IVerifier.sol";
import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";

/// @notice EIP-7562-compatible paymaster contract for Tint, designed to work with
/// the v0.8.0 EntryPoint contract.
///
/// @dev This paymaster intentionally only supports WETH as its fee token. This can be
/// changed if needed using a TWAP oracle.
contract TintPaymaster is Tint, BasePaymaster {
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
    /// @dev The amount of gas required to execute postOp. Actual gas may be lower.
    uint128 public constant POST_OP_GAS_COST = 100_000;
    uint128 public constant MIN_REFUND = 10_000 gwei;

    constructor(IEntryPoint _entryPoint, IVerifier _verifier, IERC20 _weth) BasePaymaster(_entryPoint) Tint(_verifier) {
        WETH = _weth;
        WETH.forceApprove(address(this), type(uint256).max);
    }

    // -------------------- BasePaymaster overrides --------------------
    function _validatePaymasterUserOp(PackedUserOperation calldata userOp, bytes32, uint256 maxCost)
        internal
        virtual
        override
        returns (bytes memory context, uint256 validationData)
    {
        bytes calldata paymasterData = _unpackPaymasterData(userOp.paymasterAndData);
        PaymasterData calldata data = _asPaymasterData(paymasterData);

        require(data.sender == userOp.sender, "sender mismatch");
        _requireFeePayment(data, maxCost);

        operate(data.operation);
        return (paymasterData, 0);
    }

    function _postOp(PostOpMode, bytes calldata context, uint256 actualGasCost, uint256 actualUserOpFeePerGas)
        internal
        virtual
        override
    {
        _refund(_asPaymasterData(context), uint128(actualGasCost), uint128(actualUserOpFeePerGas));
    }

    // -------------------- Internal helpers --------------------

    function _unpackPaymasterData(bytes calldata paymasterAndData) internal pure returns (bytes calldata data) {
        require(paymasterAndData.length >= UserOperationLib.PAYMASTER_DATA_OFFSET, "paymasterAndData too short");
        return paymasterAndData[UserOperationLib.PAYMASTER_DATA_OFFSET:];
    }

    /// Reinterprets `data` as `PaymasterData calldata` with no copy, relying on `data` being
    /// exactly the ABI tuple encoding `abi.decode(data, (PaymasterData))` would expect.
    ///
    /// @dev A calldata struct is represented as a single word (the offset of its head), so this
    /// is safe as long as `PaymasterData`'s field layout matches what's encoded into `data`.
    function _asPaymasterData(bytes calldata data) internal pure returns (PaymasterData calldata paymasterData) {
        assembly {
            paymasterData := data.offset
        }
    }

    /// Requires that the operation pays at least `requiredFee` in WETH to the paymaster.
    function _requireFeePayment(PaymasterData calldata data, uint256 requiredFee) internal view {
        if (_feePaid(data) < requiredFee) {
            revert("insufficient fee paid");
        }
    }

    /// Refunds the sender the difference between the fee paid and the actual fee incurred by the
    /// paymaster.
    ///
    /// @dev Refunds the payer as a deposit into the privacy pool which can be withdrawn later.
    /// @dev Refunds are skipped if no refund commitment is provided, or if the refund is below the minimum threshold.
    function _refund(PaymasterData calldata data, uint128 actualGasCost, uint128 actualUserOpFeePerGas) internal {
        uint128 totalFeePaid = _feePaid(data);

        uint128 actualFee = actualGasCost + POST_OP_GAS_COST * actualUserOpFeePerGas;
        uint128 refund = totalFeePaid - actualFee;

        if (refund < MIN_REFUND) {
            return;
        }

        if (data.refundPartialCommitment == bytes32(0)) {
            return;
        }

        this.deposit(address(WETH), refund, data.refundPartialCommitment, data.refundPartialEncrypted);
    }

    function _feePaid(PaymasterData calldata data) internal view returns (uint128) {
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

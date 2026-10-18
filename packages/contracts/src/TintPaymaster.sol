// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SafeERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {BasePaymaster} from "@account-abstraction/contracts/core/BasePaymaster.sol";
import {UserOperationLib} from "@account-abstraction/contracts/core/UserOperationLib.sol";
import {IEntryPoint} from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import {PackedUserOperation} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";

import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {Simulatable} from "./Simulatable.sol";
import {ProofLib} from "./lib/ProofLib.sol";
import {N_WITHDRAWALS} from "./lib/Constants.sol";

/// @notice ERC-4337 paymaster that sponsors userops by executing a Tint operation
/// which unshields its fee in WETH to the paymaster. Designed for the v0.8.0
/// EntryPoint.
///
/// @dev The operation executes during validation, including nullifier writes. Because
/// this involves writing to non-entity storage, it is not compliant with eip-7562 storage
/// rules (https://eips.ethereum.org/EIPS/eip-7562#storage-rules).
/// Tint also reads `block.number` to checkpoint the active block, a banned opcode under
/// eip-7562 (OP-011).
contract TintPaymaster is BasePaymaster, Simulatable {
    using ProofLib for IPrivacyPool.Operation;
    using SafeERC20 for IERC20;

    /// Additional gas deducted from the paymaster's refund to cover EntryPoint overhead.
    /// TODO: Measure and optimize me.
    uint256 public constant POST_OP_OVERHEAD = 10_000;

    IPrivacyPool public immutable TINT;
    IERC20 public immutable WETH;

    /// The kind of refund performed in `postOP`.
    enum RefundKind {
        /// The paymaster keeps the whole fee.
        None,
        /// Transfers the refund to `recipient`.
        Transfer,
        /// Deposits the refund into Tint as a note for `partialCommitment`.
        Deposit
    }

    /// Where the unspent fee goes.
    struct Refund {
        RefundKind kind;
        address recipient;
        bytes32 partialCommitment;
        bytes encryptedPartial;
    }

    error InvalidBinding();
    error InsufficientFee(uint256 required, uint256 paid);

    constructor(IEntryPoint _entryPoint, IPrivacyPool _tint, IERC20 _weth) BasePaymaster(_entryPoint) {
        TINT = _tint;
        WETH = _weth;
        _weth.forceApprove(address(_tint), type(uint256).max);
    }

    /// Withdraws `amount` of WETH from the paymaster to `to`.
    function sweep(address to, uint256 amount) external onlyOwner {
        WETH.safeTransfer(to, amount);
    }

    /// The binding an operation must carry to pay for `sender`'s userop with `refund`.
    function binding(address sender, Refund memory refund) public pure returns (bytes32) {
        return keccak256(abi.encode(sender, refund));
    }

    /// Ensures the operation pays for the userop and executes it on Tint.
    ///
    /// Returns context for `postOp` if a refund is requested.
    function _validatePaymasterUserOp(PackedUserOperation calldata userOp, bytes32, uint256 maxCost)
        internal
        virtual
        override
        returns (bytes memory context, uint256 validationData)
    {
        (IPrivacyPool.Operation calldata op, Refund calldata refund) = _decode(userOp.paymasterAndData);

        if (op.context.binding != binding(userOp.sender, refund)) revert InvalidBinding();

        uint256 fee = _feePaid(op);
        if (fee < maxCost && !_simulating()) revert InsufficientFee(maxCost, fee);

        // Involves a non-compliant storage write to the nullifier set, required
        // to prevent double-spending.
        TINT.operate(op);

        //? An empty context skips `postOp`.
        if (refund.kind == RefundKind.None) return ("", 0);
        uint256 postOpGas = UserOperationLib.unpackPostOpGasLimit(userOp) + POST_OP_OVERHEAD;
        return (abi.encode(fee, postOpGas, refund), 0);
    }

    /// Refunds the fee left over after gas.
    function _postOp(PostOpMode, bytes calldata context, uint256 actualGasCost, uint256 actualUserOpFeePerGas)
        internal
        virtual
        override
    {
        (uint256 fee, uint256 postOpGas, Refund memory refund) = abi.decode(context, (uint256, uint256, Refund));

        uint256 cost = actualGasCost + postOpGas * actualUserOpFeePerGas;
        if (fee <= cost) {
            //? While simulating refund anyway so gas estimates cover refunds.
            if (!_simulating()) return;
            cost = 0;
        }

        uint256 amount = fee - cost;
        if (refund.kind == RefundKind.Transfer) {
            WETH.safeTransfer(refund.recipient, amount);
            return;
        }

        // SAFETY: Amount can never exceed the amount of WETH unshielded.
        // forge-lint: disable-next-line(unsafe-typecast)
        TINT.deposit(address(WETH), uint128(amount), refund.partialCommitment, refund.encryptedPartial);
    }

    /// Reinterprets the paymaster data as an `(Operation, Refund) calldata`.
    function _decode(bytes calldata paymasterAndData)
        internal
        pure
        returns (IPrivacyPool.Operation calldata op, Refund calldata refund)
    {
        bytes calldata data = paymasterAndData[UserOperationLib.PAYMASTER_DATA_OFFSET:];
        assembly {
            op := add(data.offset, calldataload(data.offset))
            refund := add(data.offset, calldataload(add(data.offset, 0x20)))
        }
    }

    /// Sums the WETH the operation unshields to this paymaster.
    function _feePaid(IPrivacyPool.Operation calldata op) internal view returns (uint256 fee) {
        for (uint256 i = 0; i < N_WITHDRAWALS; i++) {
            if (op.unshieldAsset(i) != address(WETH)) continue;
            if (op.unshieldRecipient(i) != address(this)) continue;
            fee += op.unshieldAmount(i);
        }
    }
}

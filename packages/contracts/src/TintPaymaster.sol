// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SafeERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {BasePaymaster} from "@account-abstraction/contracts/core/BasePaymaster.sol";
import {UserOperationLib} from "@account-abstraction/contracts/core/UserOperationLib.sol";
import {IEntryPoint} from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import {PackedUserOperation} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";

import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {ProofLib} from "./lib/ProofLib.sol";
import {N_WITHDRAWALS} from "./lib/Constants.sol";

/// @notice ERC-4337 paymaster that sponsors userops by executing a Tint operation
/// which unshields its fee in WETH to the paymaster. Designed for the v0.8.0
/// EntryPoint.
///
/// @dev The operation executes during validation, including nullifier writes. Because
/// this involves writing to non-entity storage, it is not compliant with eip-7562 storage
/// rules (https://eips.ethereum.org/EIPS/eip-7562#storage-rules).
contract TintPaymaster is BasePaymaster {
    using ProofLib for IPrivacyPool.Operation;
    using SafeERC20 for IERC20;

    IPrivacyPool public immutable TINT;
    IERC20 public immutable WETH;

    error InvalidBinding();
    error InsufficientFee(uint256 required, uint256 paid);

    constructor(IEntryPoint _entryPoint, IPrivacyPool _tint, IERC20 _weth) BasePaymaster(_entryPoint) {
        TINT = _tint;
        WETH = _weth;
    }

    function sweep(address to, uint256 amount) external onlyOwner {
        WETH.safeTransfer(to, amount);
    }

    function _validatePaymasterUserOp(PackedUserOperation calldata userOp, bytes32, uint256 maxCost)
        internal
        virtual
        override
        returns (bytes memory context, uint256 validationData)
    {
        IPrivacyPool.Operation calldata op = _asOperation(userOp.paymasterAndData);

        if (op.context.binding != bytes32(uint256(uint160(userOp.sender)))) revert InvalidBinding();

        uint256 fee = _feePaid(op);
        if (fee < maxCost) revert InsufficientFee(maxCost, fee);

        // Involves a non-compliant storage write to the nullifier set, required
        // to prevent double-spending.
        TINT.operate(op);
        return ("", 0);
    }

    /// Reinterprets the paymaster data as an `Operation calldata`.
    ///
    /// @dev The data is `abi.encode(op)`, whose first word is the offset of the
    /// (dynamic) operation tuple.
    function _asOperation(bytes calldata paymasterAndData) internal pure returns (IPrivacyPool.Operation calldata op) {
        bytes calldata data = paymasterAndData[UserOperationLib.PAYMASTER_DATA_OFFSET:];
        assembly {
            op := add(data.offset, calldataload(data.offset))
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

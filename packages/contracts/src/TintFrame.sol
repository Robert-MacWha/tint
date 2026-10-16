// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Tint} from "./Tint.sol";
import {IVerifier} from "./interfaces/IVerifier.sol";
import {IPoseidon2T3} from "./interfaces/IPoseidon2T3.sol";
import {IPrivacyPool} from "./interfaces/IPrivacyPool.sol";
import {IWETH} from "./interfaces/IWETH.sol";
import {ProofLib} from "./lib/ProofLib.sol";
import {N_INPUTS, N_WITHDRAWALS} from "./lib/Constants.sol";
import {IFrameOpcodes, TxParam, FrameParam, SigParam, FrameMode, SignatureScheme} from "./frame/IFrameOpcodes.sol";

/// @notice Tint as an EIP-8141 frame-transaction sender, so operations can pay their
/// own gas from shielded WETH. A sponsored transaction's frames are:
///
///   0. VERIFY(this, `validate()`): checks the frames and the operation, then APPROVEs
///      execution and payment, which debits `max_cost` from this contract's ETH float.
///   1. SENDER(this, `executeSponsored(op)`): executes the operation and unwraps the WETH
///      fee it unshielded to this contract, refilling the float.
///   2.. any non-SENDER frames. DEFAULT frames are called by the ENTRY_POINT, so they
///      can't move the pool's assets.
///
/// @dev Deploy the bytecode from `just build-frame`, which rewrites the {_approve} marker
/// into the APPROVE opcode. Solc can't emit APPROVE, and it must run in this contract's
/// own code because ethrex rejects DELEGATECALL inside a VERIFY frame.
///
/// @dev The fee (1:1 WETH:ETH) must cover `max_cost`; whatever the transaction doesn't
/// use stays here as ETH. A SENDER frame that fails after approval costs the float
/// `max_cost`, so frame 1's gas limits are floored and validation re-checks everything
/// `_executeOperation` depends on.
contract TintFrame is Tint {
    using ProofLib for IPrivacyPool.Operation;

    uint256 public constant MIN_EXECUTE_GAS = 1_000_000;
    uint256 public constant MIN_EXECUTE_STATE_GAS = 2_500_000;

    IWETH public immutable WETH;
    IFrameOpcodes public immutable FRAME;

    error InvalidFrames();
    error InvalidBinding();
    error PolicyNotSupported();
    error InsufficientFee();
    error OnlySelf();

    constructor(IVerifier _verifier, IPoseidon2T3 _poseidon, IWETH _weth, IFrameOpcodes _frame)
        Tint(_verifier, _poseidon)
    {
        WETH = _weth;
        FRAME = _frame;
    }

    /// @notice Accepts the float, and WETH unwraps.
    receive() external payable {}

    /// @notice VERIFY entrypoint (frame 0). Approves execution and payment if the
    /// transaction is a well-formed sponsored operation.
    function validate() external {
        _requireFrames();

        // Frame 1 calls `executeSponsored(op)`. Re-point its calldata at `checkSponsored`,
        // which shares the signature, to verify `op` without copying it into this frame.
        bytes memory data = FRAME.frameData(1);
        if (data.length < 4 || bytes4(data) != this.executeSponsored.selector) revert InvalidFrames();
        bytes4 selector = this.checkSponsored.selector;
        assembly ("memory-safe") {
            let p := add(data, 0x20)
            mstore(p, or(shr(32, shl(32, mload(p))), selector))
        }
        (bool ok, bytes memory err) = address(this).staticcall(data);
        if (!ok) {
            assembly ("memory-safe") {
                revert(add(err, 0x20), mload(err))
            }
        }

        _approve();
    }

    /// @notice Checks `op` is valid, spends only notes without a policy, is bound to
    /// signature 0, and pays at least `max_cost`.
    /// @dev `op.context.binding` is the address of signature 0's signer.
    function checkSponsored(IPrivacyPool.Operation calldata op) external view {
        _requireNoPolicies(op);
        verifyOperation(op);

        // Signature 0 has an empty `msg`, so it covers the whole transaction. The protocol
        // has already checked it.
        uint256 binding = uint256(op.context.binding);
        if (
            binding == 0 || FRAME.sigParam(0, SigParam.SIGNER) != binding
                || FRAME.sigParam(0, SigParam.SCHEME) != SignatureScheme.SECP256K1
                || FRAME.sigParam(0, SigParam.MSG) != 0
        ) {
            revert InvalidBinding();
        }

        if (_feePaid(op) < FRAME.txParam(TxParam.MAX_COST)) revert InsufficientFee();
    }

    /// @notice SENDER entrypoint (frame 1). Executes the operation `validate` verified.
    /// @dev Doesn't re-verify: frame 0 checked these exact bytes and no frame runs in
    /// between. `_spend` still rejects spent nullifiers.
    function executeSponsored(IPrivacyPool.Operation calldata op) external {
        if (msg.sender != address(this)) revert OnlySelf();
        _executeOperation(op);
        WETH.withdraw(_feePaid(op));
    }

    /// @dev Requires this to be frame 0, frame 1 to be the `executeSponsored` SENDER frame,
    /// and no later frame to act as this contract.
    function _requireFrames() internal view {
        if (FRAME.txParam(TxParam.CURRENT_FRAME) != 0) revert InvalidFrames();

        if (
            FRAME.frameParam(1, FrameParam.MODE) != FrameMode.SENDER
                || FRAME.frameParam(1, FrameParam.TARGET) != uint160(address(this))
                || FRAME.frameParam(1, FrameParam.FLAGS) != 0 || FRAME.frameParam(1, FrameParam.VALUE) != 0
                || FRAME.frameParam(1, FrameParam.GAS_LIMIT) < MIN_EXECUTE_GAS
                || FRAME.frameParam(1, FrameParam.STATE_GAS_LIMIT) < MIN_EXECUTE_STATE_GAS
        ) {
            revert InvalidFrames();
        }

        uint256 frames = FRAME.txParam(TxParam.FRAME_COUNT);
        for (uint256 i = 2; i < frames; ++i) {
            if (FRAME.frameParam(i, FrameParam.MODE) == FrameMode.SENDER) revert InvalidFrames();
        }
    }

    /// @dev Requires every input of `op` to have no policy and no policy input.
    function _requireNoPolicies(IPrivacyPool.Operation calldata op) internal pure {
        for (uint256 i; i < N_INPUTS; ++i) {
            if (op.policyAddress(i) != address(0) || op.context.policyInputs[i].length != 0) {
                revert PolicyNotSupported();
            }
        }
    }

    /// @dev The WETH `op` unshields to this contract.
    function _feePaid(IPrivacyPool.Operation calldata op) internal view returns (uint256 fee) {
        for (uint256 i; i < N_WITHDRAWALS; ++i) {
            if (op.unshieldAsset(i) != address(WETH) || op.unshieldRecipient(i) != address(this)) continue;
            fee += op.unshieldAmount(i);
        }
    }

    /// @dev APPROVE(offset 0, length 0, scope 3 = execution and payment). `just build-frame`
    /// rewrites this exact `sstore` into the opcode; unpatched builds (tests) record it.
    function _approve() internal {
        assembly {
            sstore(0x7f40b94fdfe5e13e90a14e3049fcb105a7f986f289ec0e6751cc92850ad1121b, 3)
        }
    }
}

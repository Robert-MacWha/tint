/// @title FrameOpcodes
/// @notice Exposes EIP-8141's frame-introspection opcodes, which solc cannot emit, as
///         ABI-callable view functions. Adapted from hegota-minimal-erc20-paymaster's
///         OpcodeLib.yul, with stack orders as in minimal-shielded-pool's dispatcher.
/// @dev Stateless and reached by STATICCALL. ethrex rejects DELEGATECALL inside a VERIFY
///      frame, so APPROVE can't live here: it must execute in the frame target's own code
///      (see TintFrame). `verbatim` puts its first argument on top of the stack.
object "FrameOpcodes" {
    code {
        datacopy(0, dataoffset("runtime"), datasize("runtime"))
        return(0, datasize("runtime"))
    }
    object "runtime" {
        code {
            if lt(calldatasize(), 4) { revert(0, 0) }

            switch shr(224, calldataload(0))

            // txParam(uint256) -> 0x16525c7f. TXPARAM (0xB0). Stack: [param].
            case 0x16525c7f {
                mstore(0, verbatim_1i_1o(hex"B0", calldataload(4)))
                return(0, 32)
            }

            // frameParam(uint256,uint256) -> 0xff846610. FRAMEPARAM (0xB3). Stack: [frameIndex, param].
            case 0xff846610 {
                mstore(0, verbatim_2i_1o(hex"B3", calldataload(4), calldataload(36)))
                return(0, 32)
            }

            // sigParam(uint256,uint256) -> 0xf457766f. SIGPARAM (0xB4). Stack: [signatureIndex, param].
            case 0xf457766f {
                mstore(0, verbatim_2i_1o(hex"B4", calldataload(4), calldataload(36)))
                return(0, 32)
            }

            // frameData(uint256) -> 0xb6481b8f. Returns the frame's whole data as ABI `bytes`.
            // FRAMEPARAM 0x04 is its length; FRAMEDATACOPY (0xB2) stack: [memOffset, dataOffset, length, frameIndex].
            case 0xb6481b8f {
                let frameIndex := calldataload(4)
                let length := verbatim_2i_1o(hex"B3", frameIndex, 0x04)
                mstore(0, 0x20)
                mstore(0x20, length)
                mstore(add(0x40, length), 0)
                verbatim_4i_0o(hex"B2", 0x40, 0, length, frameIndex)
                return(0, add(0x40, and(add(length, 31), not(31))))
            }

            default { revert(0, 0) }
        }
    }
}

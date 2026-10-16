// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @notice TXPARAM (0xB0) parameters, as ethrex's hegota-testnet numbers them.
library TxParam {
    uint256 internal constant MAX_COST = 0x06;
    uint256 internal constant FRAME_COUNT = 0x09;
    uint256 internal constant CURRENT_FRAME = 0x0A;
}

/// @notice FRAMEPARAM (0xB3) parameters, as ethrex's hegota-testnet numbers them.
library FrameParam {
    uint256 internal constant TARGET = 0x00;
    uint256 internal constant GAS_LIMIT = 0x01;
    uint256 internal constant MODE = 0x02;
    uint256 internal constant FLAGS = 0x03;
    uint256 internal constant VALUE = 0x08;
    uint256 internal constant STATE_GAS_LIMIT = 0x09;
}

/// @notice SIGPARAM (0xB4) parameters, as ethrex's hegota-testnet numbers them.
library SigParam {
    uint256 internal constant SIGNER = 0x00;
    uint256 internal constant SCHEME = 0x01;
    uint256 internal constant MSG = 0x02;
}

library FrameMode {
    uint256 internal constant DEFAULT = 0;
    uint256 internal constant VERIFY = 1;
    uint256 internal constant SENDER = 2;
}

library SignatureScheme {
    uint256 internal constant ARBITRARY = 0;
    uint256 internal constant SECP256K1 = 1;
    uint256 internal constant P256 = 2;
}

/// @notice ABI of `FrameOpcodes.yul`.
interface IFrameOpcodes {
    function txParam(uint256 param) external view returns (uint256);
    function frameParam(uint256 frameIndex, uint256 param) external view returns (uint256);
    function sigParam(uint256 signatureIndex, uint256 param) external view returns (uint256);
    function frameData(uint256 frameIndex) external view returns (bytes memory);
}

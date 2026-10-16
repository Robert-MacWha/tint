// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IFrameOpcodes} from "../../src/frame/IFrameOpcodes.sol";

/// @notice Stands in for `FrameOpcodes.yul` outside a frame transaction, returning
/// whatever transaction, frame, and signature parameters the test sets.
contract MockFrameOpcodes is IFrameOpcodes {
    mapping(uint256 param => uint256) internal txParams;
    mapping(uint256 frameIndex => mapping(uint256 param => uint256)) internal frameParams;
    mapping(uint256 signatureIndex => mapping(uint256 param => uint256)) internal sigParams;
    mapping(uint256 frameIndex => bytes) internal frameDatas;

    function setTxParam(uint256 param, uint256 value) external {
        txParams[param] = value;
    }

    function setFrameParam(uint256 frameIndex, uint256 param, uint256 value) external {
        frameParams[frameIndex][param] = value;
    }

    function setSigParam(uint256 signatureIndex, uint256 param, uint256 value) external {
        sigParams[signatureIndex][param] = value;
    }

    function setFrameData(uint256 frameIndex, bytes calldata data) external {
        frameDatas[frameIndex] = data;
    }

    function txParam(uint256 param) external view returns (uint256) {
        return txParams[param];
    }

    function frameParam(uint256 frameIndex, uint256 param) external view returns (uint256) {
        return frameParams[frameIndex][param];
    }

    function sigParam(uint256 signatureIndex, uint256 param) external view returns (uint256) {
        return sigParams[signatureIndex][param];
    }

    function frameData(uint256 frameIndex) external view returns (bytes memory) {
        return frameDatas[frameIndex];
    }
}

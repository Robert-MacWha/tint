// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Tint} from "../src/Tint.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {N_CONST, N_INPUTS, N_OUTPUTS, FRONTIER_OFFSET, BN254_FR_MODULUS} from "../src/lib/Constants.sol";

/// @notice Builds memory `Operation`s for tests, writing typed fields into
/// their `pub` slots.
library OperationLib {
    /// @dev An operation bound to `tint`'s live frontier.
    function bind(Tint tint) internal view returns (IPrivacyPool.Operation memory op) {
        bytes32[] memory frontier = tint.frontier();
        op.pub = new bytes32[](FRONTIER_OFFSET + frontier.length);
        op.pub[2] = bytes32(tint.packedRanks());
        op.pub[3] = bytes32(tint.depth());
        op.pub[4] = bytes32(tint.count());
        for (uint256 i = 0; i < frontier.length; ++i) {
            op.pub[FRONTIER_OFFSET + i] = frontier[i];
        }
    }

    /// @dev Binds `pub[0]` to `op.context`. Call after the last `context` change.
    function seal(IPrivacyPool.Operation memory op) internal pure returns (IPrivacyPool.Operation memory) {
        op.pub[0] = bytes32(uint256(keccak256(abi.encode(op.context))) % BN254_FR_MODULUS);
        return op;
    }

    function frontierRoot(IPrivacyPool.Operation memory op, uint256 i) internal pure returns (bytes32) {
        return op.pub[FRONTIER_OFFSET + i];
    }

    function setFrontierRoot(IPrivacyPool.Operation memory op, uint256 i, bytes32 v) internal pure {
        op.pub[FRONTIER_OFFSET + i] = v;
    }

    function nullifier(IPrivacyPool.Operation memory op, uint256 i) internal pure returns (bytes32) {
        return op.pub[N_CONST + 2 * i];
    }

    function setNullifier(IPrivacyPool.Operation memory op, uint256 i, bytes32 v) internal pure {
        op.pub[N_CONST + 2 * i] = v;
    }

    function setSpendabilityAddress(IPrivacyPool.Operation memory op, uint256 i, address v) internal pure {
        op.pub[N_CONST + 2 * i + 1] = bytes32(uint256(uint160(v)));
    }

    function commitmentOut(IPrivacyPool.Operation memory op, uint256 i) internal pure returns (bytes32) {
        return op.pub[N_CONST + 2 * N_INPUTS + i];
    }

    function setCommitmentOut(IPrivacyPool.Operation memory op, uint256 i, bytes32 v) internal pure {
        op.pub[N_CONST + 2 * N_INPUTS + i] = v;
    }

    function unshieldAmount(IPrivacyPool.Operation memory op, uint256 i) internal pure returns (uint128) {
        return uint128(uint256(op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i]));
    }

    function setUnshieldAmount(IPrivacyPool.Operation memory op, uint256 i, uint128 v) internal pure {
        op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i] = bytes32(uint256(v));
    }

    function unshieldAsset(IPrivacyPool.Operation memory op, uint256 i) internal pure returns (address) {
        return address(uint160(uint256(op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i + 1])));
    }

    function setUnshieldAsset(IPrivacyPool.Operation memory op, uint256 i, address v) internal pure {
        op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i + 1] = bytes32(uint256(uint160(v)));
    }

    function unshieldRecipient(IPrivacyPool.Operation memory op, uint256 i) internal pure returns (address) {
        return address(uint160(uint256(op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i + 2])));
    }

    function setUnshieldRecipient(IPrivacyPool.Operation memory op, uint256 i, address v) internal pure {
        op.pub[N_CONST + 2 * N_INPUTS + N_OUTPUTS + 3 * i + 2] = bytes32(uint256(uint160(v)));
    }
}

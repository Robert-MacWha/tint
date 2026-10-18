// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPrivacyPool} from "./IPrivacyPool.sol";

interface IPolicy {
    /// @notice Reverts unless `operation` is authorized under this policy.
    /// @dev Given the policy's placeholder input, this must spend the same gas
    ///      as for a valid input before reverting.
    function requireAuthorized(IPrivacyPool.Operation calldata operation) external view;
}

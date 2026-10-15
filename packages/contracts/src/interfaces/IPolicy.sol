// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPrivacyPool} from "./IPrivacyPool.sol";

interface IPolicy {
    function requireAuthorized(IPrivacyPool.Operation calldata operation) external view;
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IPrivacyPool} from "./IPrivacyPool.sol";

interface ISpendability {
    function isSpendable(
        IPrivacyPool.Operation calldata operation
    ) external view returns (bool);
}

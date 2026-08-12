// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IPrivacyPool} from "./IPrivacyPool.sol";

interface ISpendability {
    // Replace ISpendability.isSpendable with ISpendability.requireSpendable, to remove the InvalidSpendability error and conditional.
    function isSpendable(
        IPrivacyPool.Operation calldata operation
    ) external view returns (bool);
}

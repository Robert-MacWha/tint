// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @notice Lets an inheriting contract skip checks that fail while being
/// simulated, so callers can estimate gas before they can pass them.
abstract contract Simulatable {
    /// @dev Slot `erc7201:tint.simulation`. This should NEVER be nonzero.
    bytes32 private constant SIMULATION_SLOT = 0xb4a234b71e30e41b0c32030fc0a6277a036950341675d2266bc10f1fc28e5200;

    /// @dev Whether the simulation flag is set. Checks should only branch on failures,
    ///      so that simulated gas costs are accurate and non-simulated calls don't
    ///      have to pay for the SIMULATION_SLOT read.
    function _simulating() internal view returns (bool simulating) {
        assembly {
            simulating := iszero(iszero(sload(SIMULATION_SLOT)))
        }
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script} from "forge-std/Script.sol";
import {console} from "forge-std/console.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {Groth16Verifier} from "../src/Groth16Verifier.sol";

/// @notice ERC20 with an open mint, for local dev/testing only.
contract MockToken is ERC20 {
    constructor() ERC20("Mock", "MCK") {
        _mint(msg.sender, 1_000_000 ether);
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// @notice Deploys Tint with the real Groth16Verifier + MockToken for local
/// dev (anvil) use. Not for production — the verifying key comes from a
/// dev-only trusted setup, not a real ceremony.
contract Deploy is Script {
    function run() external {
        vm.startBroadcast();

        Groth16Verifier verifier = new Groth16Verifier();
        Tint tint = new Tint(address(verifier));
        MockToken token = new MockToken();

        vm.stopBroadcast();

        console.log("Groth16Verifier:", address(verifier));
        console.log("Tint:", address(tint));
        console.log("MockToken:", address(token));
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {IEntryPoint} from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

import {TintPaymaster} from "../src/TintPaymaster.sol";
import {Tint} from "../src/Tint.sol";
import {Deploy} from "./Deploy.s.sol";

/// Deploys a TintPaymaster owned by the broadcaster over the deterministic Tint (deploying
/// it first if needed), funding its EntryPoint deposit and stake. The paymaster takes its
/// owner from `msg.sender`, so it isn't deployed deterministically. Requires TOKEN (the
/// fee WETH). `--slow` keeps the calls from landing before the paymaster is deployed. Usage:
///   forge script script/DeployPaymaster.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast --slow
contract DeployPaymaster is Script {
    IEntryPoint internal constant ENTRY_POINT = IEntryPoint(0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108);
    uint256 internal constant DEPOSIT = 1 ether;
    uint32 internal constant UNSTAKE_DELAY_SEC = 3600;

    function run() external returns (TintPaymaster paymaster) {
        (, Tint tint) = new Deploy().run();
        IERC20 weth = IERC20(vm.envAddress("TOKEN"));

        vm.startBroadcast();
        paymaster = new TintPaymaster(ENTRY_POINT, tint, weth);
        paymaster.deposit{value: DEPOSIT}();
        paymaster.addStake{value: DEPOSIT}(UNSTAKE_DELAY_SEC);
        vm.stopBroadcast();

        console.log("TintPaymaster deployed to", address(paymaster));
    }
}

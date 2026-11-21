// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {TintAdapter} from "../src/TintAdapter.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";

/// Deploys the TintAdapter over the Tint deployment at `TINT_ADDRESS`. Usage:
///   TINT_ADDRESS=0x... forge script script/DeployAdapter.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployAdapter is Script {
    function run() external returns (TintAdapter adapter) {
        IPrivacyPool tint = IPrivacyPool(vm.envAddress("TINT_ADDRESS"));

        vm.startBroadcast();
        adapter = new TintAdapter(tint);
        vm.stopBroadcast();

        console.log("TintAdapter deployed to", address(adapter));
    }
}

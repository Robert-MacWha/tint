// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IEntryPoint} from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {Tint} from "../src/Tint.sol";
import {TintAdapter} from "../src/TintAdapter.sol";
import {TintPaymaster} from "../src/TintPaymaster.sol";
import {PasswordPolicy} from "../src/policy/password/PasswordPolicy.sol";
import {MultisigPolicy} from "../src/policy/multisig/MultisigPolicy.sol";
import {HardwareWalletPolicy} from "../src/policy/hardware-wallet/HardwareWalletPolicy.sol";
import {TimelockPolicy} from "../src/policy/timelock/TimelockPolicy.sol";
import {IntentPolicy} from "../src/policy/intent/IntentPolicy.sol";
import {deployTint} from "./Deploy.s.sol";
import {deployPasswordPolicy} from "./DeployPasswordPolicy.s.sol";
import {deployMultisigPolicy} from "./DeployMultisigPolicy.s.sol";
import {deployHardwareWalletPolicy} from "./DeployHardwareWalletPolicy.s.sol";
import {deployTimelockPolicy} from "./DeployTimelockPolicy.s.sol";
import {deployIntentPolicy} from "./DeployIntentPolicy.s.sol";

/// Deploys everything a testnet needs: Poseidon, Tint, its paymaster and adapter, and all policies.
/// `WETH_ADDRESS` is the paymaster's fee token and `ENTRY_POINT_ADDRESS` defaults to the canonical
/// ERC-4337 v0.8 EntryPoint. The paymaster still needs an EntryPoint deposit and stake. Usage:
///   WETH_ADDRESS=0x... forge script script/DeployTestnet.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
contract DeployTestnet is Script {
    address constant ENTRY_POINT_V08 = 0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108;

    function run() external {
        IEntryPoint entryPoint = IEntryPoint(vm.envOr("ENTRY_POINT_ADDRESS", ENTRY_POINT_V08));
        IERC20 weth = IERC20(vm.envAddress("WETH_ADDRESS"));

        vm.startBroadcast();

        Poseidon2T2 poseidonT2 = new Poseidon2T2();
        Poseidon2T3 poseidonT3 = new Poseidon2T3();
        (, Tint tint) = deployTint(poseidonT3);
        TintPaymaster paymaster = new TintPaymaster(entryPoint, tint, weth);
        TintAdapter adapter = new TintAdapter(tint);
        (, PasswordPolicy password) = deployPasswordPolicy(poseidonT2);
        (, MultisigPolicy multisig) = deployMultisigPolicy(poseidonT2);
        (, HardwareWalletPolicy hardwareWallet) = deployHardwareWalletPolicy(poseidonT2);
        (, TimelockPolicy timelock) = deployTimelockPolicy(poseidonT2);
        (, IntentPolicy intent) = deployIntentPolicy(poseidonT2);

        vm.stopBroadcast();

        // Logged as root `justfile` variables, ready to paste.
        _log("TINT_ADDRESS", address(tint));
        _log("ADAPTER_ADDRESS", address(adapter));
        _log("PAYMASTER_ADDRESS", address(paymaster));
        _log("PASSWORD_POLICY_ADDRESS", address(password));
        _log("MULTISIG_POLICY_ADDRESS", address(multisig));
        _log("HARDWARE_WALLET_POLICY_ADDRESS", address(hardwareWallet));
        _log("TIMELOCK_POLICY_ADDRESS", address(timelock));
        _log("INTENT_POLICY_ADDRESS", address(intent));
    }

    function _log(string memory name, address addr) internal pure {
        console.log(string.concat(name, ' := "', vm.toString(addr), '"'));
    }
}

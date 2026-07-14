// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {console} from "forge-std/console.sol";
import {Test} from "forge-std/Test.sol";
import {LibPoseidon2Yul} from "poseidon2-evm/src/bn254/yul/LibPoseidon2Yul.sol";

contract PoseidonGasReportTest is Test {
    function test_poseidon2_gas() public {
        uint256 gasBefore = gasleft();

        LibPoseidon2Yul.hash_2(1, 2);

        uint256 gasUsed = gasBefore - gasleft();
        console.log("Gas used:", gasUsed);
    }
}

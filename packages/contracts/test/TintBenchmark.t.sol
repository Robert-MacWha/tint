// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {Tint} from "../src/Tint.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MockToken is ERC20 {
    constructor() ERC20("Mock", "MCK") {
        _mint(msg.sender, type(uint128).max);
    }
}

contract MockVerifier {
    function verifyProof(
        uint256[2] calldata,
        uint256[2][2] calldata,
        uint256[2] calldata,
        uint[44] memory
    ) external pure returns (bool) {
        return true;
    }
}

contract MockArchiveVerifier {
    function verifyProof(
        uint256[2] calldata,
        uint256[2][2] calldata,
        uint256[2] calldata,
        uint[3] memory
    ) external pure returns (bool) {
        return true;
    }
}

contract TintGasReportTest is Test {
    Tint public tint;
    MockToken public token;

    function setUp() public {
        token = new MockToken();
        MockVerifier verifier = new MockVerifier();
        tint = new Tint(address(verifier), address(new MockArchiveVerifier()));
        token.approve(address(tint), type(uint256).max);
    }

    function test_shield_gas() public {
        for (uint256 i = 1; i <= 250; i++) {
            tint.shield(address(token), 1, uint256(i));
        }
    }
}

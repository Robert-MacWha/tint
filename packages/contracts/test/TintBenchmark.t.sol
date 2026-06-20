// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {AGGREGATION_RING_SIZE} from "../src/lib/Constants.sol";

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
        uint256[24] memory
    ) external pure returns (bool) {
        return true;
    }
}

contract TestTint is Tint {
    constructor(address _verifier) Tint(_verifier) {}

    /// @dev Warms all storage slots touched by deposit() without calling deposit().
    /// This prevents warm-up writes from appearing in Forge's gas report for deposit().
    function warmStorage() external {
        for (uint256 i = 0; i < AGGREGATION_RING_SIZE; i++) {
            aggregationHashRing[i] = bytes32(uint256(i + 1));
        }
        totalStaged = AGGREGATION_RING_SIZE;
        totalConsumed = AGGREGATION_RING_SIZE;
    }
}

contract TintGasReportTest is Test {
    TestTint public tint;
    MockToken public token;

    function setUp() public {
        token = new MockToken();
        MockVerifier verifier = new MockVerifier();
        tint = new TestTint(address(verifier));
        token.approve(address(tint), type(uint256).max);
    }

    function test_shield_gas() public {
        tint.warmStorage();

        for (uint256 i = 0; i < 2; i++) {
            tint.deposit(
                address(token),
                1,
                bytes32(uint256(keccak256(abi.encode(i))))
            );
        }
    }
}

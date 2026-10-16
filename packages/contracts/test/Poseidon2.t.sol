// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {Poseidon2T2_BN254} from "@taceo/poseidon2/Poseidon2T2_BN254.sol";
import {Poseidon2T3_BN254} from "@taceo/poseidon2/Poseidon2T3_BN254.sol";
import {Poseidon2T2} from "../src/poseidon/Poseidon2T2.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {BN254_FR_MODULUS} from "../src/lib/Constants.sol";

/// Checks the standalone Poseidon2 contracts against the taceo library they wrap.
contract Poseidon2Test is Test {
    Poseidon2T2 t2;
    Poseidon2T3 t3;

    function setUp() public {
        t2 = new Poseidon2T2();
        t3 = new Poseidon2T3();
    }

    function test_t2MatchesLibrary(uint256 x, uint256 y, uint256 domainSep) public view {
        x = bound(x, 0, BN254_FR_MODULUS - 1);
        y = bound(y, 0, BN254_FR_MODULUS - 1);
        domainSep = bound(domainSep, 0, BN254_FR_MODULUS - 1);
        assertEq(t2.compress(x, y, domainSep), Poseidon2T2_BN254.compress([x, y], domainSep));
    }

    function test_t3MatchesLibrary(uint256 x, uint256 y, uint256 z, uint256 domainSep) public view {
        x = bound(x, 0, BN254_FR_MODULUS - 1);
        y = bound(y, 0, BN254_FR_MODULUS - 1);
        z = bound(z, 0, BN254_FR_MODULUS - 1);
        domainSep = bound(domainSep, 0, BN254_FR_MODULUS - 1);
        assertEq(t3.compress(x, y, z, domainSep), Poseidon2T3_BN254.compress([x, y, z], domainSep));
    }

    function test_t2NonCanonicalInput_reverts() public {
        vm.expectRevert(Poseidon2T2_BN254.NotInPrimefield.selector);
        t2.compress(BN254_FR_MODULUS, 0, 0);
    }

    function test_t3NonCanonicalInput_reverts() public {
        vm.expectRevert(Poseidon2T3_BN254.NotInPrimefield.selector);
        t3.compress(0, 0, BN254_FR_MODULUS, 0);
    }
}

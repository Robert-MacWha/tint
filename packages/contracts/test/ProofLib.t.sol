// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ProofLib} from "../src/lib/ProofLib.sol";

contract ProofLibTests is Test {
    /// @notice Ground truth captured from `tint_rs::note::commitment::asset_to_fr`
    /// for these same addresses, to confirm the Solidity port replicates its
    /// little-endian byte order exactly.
    function test_assetToFr_matchesRustEncoding() public pure {
        assertEq(
            ProofLib.assetToFr(0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9),
            1152994189795012287253187359815331965862836004060
        );
        assertEq(ProofLib.assetToFr(address(0)), 0);
    }
}

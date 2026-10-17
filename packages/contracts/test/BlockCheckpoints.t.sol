// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {BlockCheckpoints} from "../src/BlockCheckpoints.sol";

contract BlockCheckpointsHarness is BlockCheckpoints {
    function checkpoint() external {
        _checkpoint();
    }
}

contract BlockCheckpointsTests is Test {
    BlockCheckpointsHarness checkpoints;

    function setUp() public {
        checkpoints = new BlockCheckpointsHarness();
    }

    /// The first checkpoint in a block should record it and emit the previous checkpoints.
    function test_checkpoint() public {
        checkpoints.checkpoint();
        uint64[4] memory previous = _levels();
        vm.roll(block.number + 10);

        vm.expectEmit();
        emit BlockCheckpoints.Checkpoint(previous);
        checkpoints.checkpoint();

        assertEq(_level(0), block.number);
    }

    /// Later checkpoints in an already-active block should be no-ops.
    function test_checkpointOncePerBlock() public {
        checkpoints.checkpoint();
        bytes memory before = abi.encode(_levels());

        vm.recordLogs();
        checkpoints.checkpoint();

        assertEq(vm.getRecordedLogs().length, 0);
        assertEq(abi.encode(_levels()), before);
    }

    /// Level `i` should only move once it's at least `100 ** i` blocks behind.
    function test_checkpointLevels() public {
        uint256 start = 2_000_000;
        vm.roll(start);
        checkpoints.checkpoint();
        assertEq(_level(0), start);
        assertEq(_level(1), start);
        assertEq(_level(2), start);
        assertEq(_level(3), start);

        vm.roll(start + 150);
        checkpoints.checkpoint();
        assertEq(_level(0), start + 150);
        assertEq(_level(1), start + 150);
        assertEq(_level(2), start);
        assertEq(_level(3), start);

        vm.roll(start + 20_000);
        checkpoints.checkpoint();
        assertEq(_level(0), start + 20_000);
        assertEq(_level(1), start + 20_000);
        assertEq(_level(2), start + 20_000);
        assertEq(_level(3), start);
    }

    function _level(uint256 i) internal view returns (uint64) {
        return checkpoints.checkpoints(i);
    }

    function _levels() internal view returns (uint64[4] memory) {
        return [_level(0), _level(1), _level(2), _level(3)];
    }
}

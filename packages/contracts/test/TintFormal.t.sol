// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {
    N_INPUTS,
    N_OUTPUTS,
    AGGREGATION_RING_SIZE
} from "../src/lib/Constants.sol";

/// Verifier stub that unconditionally accepts every proof.
/// Treats proof validity as an axiom to isolate the contract's own invariants.
contract TrustedVerifier is IVerifier {
    function verifyProof(
        uint[2] memory,
        uint[2][2] memory,
        uint[2] memory,
        uint[24] memory
    ) external pure returns (bool) {
        return true;
    }
}

contract MockToken is ERC20 {
    constructor() ERC20("Mock", "MCK") {
        _mint(msg.sender, type(uint128).max);
    }
}

/// Extends Tint with state-injection helpers for formal verification.
contract TintHarness is Tint {
    constructor(address v) Tint(v) {}

    function setCounters(uint128 staged, uint128 consumed) external {
        totalStaged = staged;
        totalConsumed = consumed;
    }

    function setRootIndex(uint128 idx) external {
        currentRootIndex = idx;
    }

    function setRootStorage(bytes32 root, uint128 idx) external {
        roots[root] = idx;
    }
}

contract TintFormal is Test {
    TintHarness tint;
    MockToken token;

    bytes32 constant SEED = bytes32(uint256(0xdeadbeef));

    function setUp() public {
        tint = new TintHarness(address(new TrustedVerifier()));
        token = new MockToken();
        token.approve(address(tint), type(uint256).max);
        // Deposit one concrete commitment so leavesAggregationIndex=0 is valid.
        tint.deposit(address(token), 1, SEED);
    }

    // ------- helpers -------

    /// Builds a single-element Operation with concrete zero nullifiers and explicit
    /// empty spendabilityData (avoids null-pointer reads in Halmos).
    function _makeOp(
        bytes32[N_INPUTS] memory nullifiers,
        bytes32 oldRoot
    ) internal pure returns (IPrivacyPool.Operation[] memory) {
        IPrivacyPool.Operation[] memory ops = new IPrivacyPool.Operation[](1);
        ops[0].oldRoot = oldRoot;
        ops[0].newRoot = bytes32(uint256(1));
        ops[0].leavesAggregationIndex = 0;
        ops[0].nullifiers = nullifiers;
        for (uint256 i; i < N_OUTPUTS; ++i) {
            ops[0].spendabilityData[i] = "";
        }
        return ops;
    }

    //     // ------- checks -------

    //     /// A nullifier can never be spent more than once.
    //     function check_doubleSpend(bytes32 n) public {
    //         vm.assume(n != 0);
    //         vm.assume(!tint.nullifierHashes(n));

    //         try tint.checkAndMarkNullifier(n) {} catch {
    //             vm.assume(false);
    //         }
    //         try tint.checkAndMarkNullifier(n) {
    //             assert(false);
    //         } catch {}
    //     }

    //     /// Once a nullifier is marked as spent, it remains spent forever.
    //     function check_nullifierPermanence(bytes32 n, bytes32 other) public {
    //         vm.assume(n != 0);
    //         vm.assume(!tint.nullifierHashes(n));

    //         try tint.checkAndMarkNullifier(n) {} catch {
    //             vm.assume(false);
    //         }
    //         try tint.checkAndMarkNullifier(other) {} catch {}
    //         assert(tint.nullifierHashes(n));
    //     }

    /// Isolation: deposit() never marks any nullifier as spent.
    function check_depositDoesNotSpendNullifier(bytes32 nullifier) public {
        vm.assume(!tint.nullifierHashes(nullifier));

        // Concrete commitment avoids symbolic Poseidon assembly evaluation.
        try tint.deposit(address(token), 1, SEED) {} catch {}
        assert(!tint.nullifierHashes(nullifier));
    }

    /// deposit() always reverts when the staging ring is full.
    ///
    /// Injects an arbitrary full state via the harness to avoid calling
    /// deposit() AGGREGATION_RING_SIZE times. The revert path is reached
    /// before Poseidon, so the concrete commitment is safe here.
    function check_stagingFullReverts(uint128 consumed) public {
        uint128 ringSize = AGGREGATION_RING_SIZE;
        vm.assume(consumed <= type(uint128).max - ringSize);
        tint.setCounters(consumed + ringSize, consumed);

        try tint.deposit(address(token), 1, SEED) {
            assert(false); // must revert with StagingFull
        } catch {}
    }

    /// Safety: operate() always reverts when oldRoot has no recorded index.
    function check_unknownRootReverts(bytes32 unknownRoot) public {
        vm.assume(tint.roots(unknownRoot) == 0);

        bytes32[N_INPUTS] memory noNullifiers;
        try tint.operate(_makeOp(noNullifiers, unknownRoot)) {
            assert(false); // must revert with InvalidOldRoot
        } catch {}
    }

    /// Invariant: currentRootIndex never decreases after any operate() call.
    function check_rootMonotonicity(uint128 startIdx, uint128 rootVal) public {
        vm.assume(startIdx >= 1);
        vm.assume(rootVal >= 1 && rootVal <= startIdx);

        tint.setRootIndex(startIdx);
        tint.setRootStorage(bytes32(0), rootVal);

        uint256 before = tint.currentRootIndex();
        bytes32[N_INPUTS] memory noNullifiers;
        try tint.operate(_makeOp(noNullifiers, bytes32(0))) {} catch {}
        assert(tint.currentRootIndex() >= before);
    }

    //     /// Liveness: a fresh, unspent nullifier can always be marked spent.
    //     function check_freshNullifierAlwaysSpendable(bytes32 n) public {
    //         vm.assume(n != 0);
    //         vm.assume(!tint.nullifierHashes(n));

    //         try tint.checkAndMarkNullifier(n) {
    //             assert(tint.nullifierHashes(n));
    //         } catch {
    //             assert(false);
    //         }
    //     }
}

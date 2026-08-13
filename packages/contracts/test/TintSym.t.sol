// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {SymTest} from "halmos-cheatcodes/SymTest.sol";
import {Test} from "forge-std/Test.sol";
import {Tint} from "../src/Tint.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ISpendability} from "../src/interfaces/ISpendability.sol";
import {MockVerifier} from "../src/mocks/MockVerifier.sol";
import {
    N_INPUTS,
    N_OUTPUTS,
    N_WITHDRAWALS,
    AGGREGATION_RING_SIZE
} from "../src/lib/Constants.sol";

/// @notice ERC20 stub whose transfers always succeed.
contract StubToken {
    function transfer(address, uint256) external pure returns (bool) {
        return true;
    }
}

/// @notice Spendability contract whose verdict is fixed per path.
contract SymSpendability is ISpendability {
    bool shouldPass;

    error NotSpendable();

    function setPass(bool v) external {
        shouldPass = v;
    }

    function requireSpendable(IPrivacyPool.Operation calldata) external view {
        if (!shouldPass) revert NotSpendable();
    }
}

contract TintHarness is Tint {
    constructor(address verifier) Tint(verifier) {}

    function setAggregationState(uint128 _consumed, uint128 _staged) public {
        consumed = _consumed;
        staged = _staged;
    }

    function setRootState(uint128 tip, bytes32 root, uint128 idx) public {
        currentRootIndex = tip;
        roots[root] = idx;
    }

    function setSpent(bytes32 hash) public {
        nullifierHashes[hash] = true;
    }

    /// @dev Override poseidon2 with cheaper keccak256. The specifics of the hash
    /// function are irrelevant to the properties being tested.
    function _hash(
        bytes32 prevHash,
        bytes32 commitment
    ) internal pure override returns (bytes32) {
        return keccak256(abi.encode(prevHash, commitment));
    }

    function executeOperation(IPrivacyPool.Operation calldata op) public {
        _executeOperation(op);
    }
}

contract TintSymTest is Test, SymTest {
    TintHarness tint;
    SymSpendability spendability;
    StubToken token;

    function setUp() public {
        token = new StubToken();
        spendability = new SymSpendability();
        tint = new TintHarness(address(new MockVerifier()));
    }

    /// Produces an arbitrary reachable pool state.
    function _arbitraryState(
        bytes32 oldRoot,
        uint128 consumed,
        uint128 staged,
        uint128 tip,
        uint128 oldRootIndex,
        bytes32 spentNullifier,
        bool spendabilityPasses
    ) internal {
        /// SAFETY: root index starts at 1 and is monotonically increasing, so 0 is unreachable.
        ///
        /// Tested by RootRegistrySymTest.check_tipNeverZero.
        vm.assume(oldRootIndex != 0);

        // SAFETY: Enforced by _commit's capacity guard.
        //
        // Tested by AggregationRingSymTest.check_commit.
        vm.assume(staged < AGGREGATION_RING_SIZE);

        // SAFETY: Overflows are not physically reachable. Avoids overflow panics
        // as false positives.
        vm.assume(uint256(consumed) + uint256(staged) < type(uint128).max);

        // SAFETY: The constructor seeds the tip at 1 and _updateRoot only ever
        // raises it, so every registered root sits at or below the tip.
        //
        // Tested by RootRegistrySymTest.
        vm.assume(tip >= 1);
        vm.assume(tip < type(uint128).max);
        vm.assume(oldRootIndex <= tip);

        tint.setAggregationState(consumed, staged);
        tint.setRootState(tip, oldRoot, oldRootIndex);
        tint.setSpent(spentNullifier);
        spendability.setPass(spendabilityPasses);
    }

    /// Narrows the operation to have a max of two inputs, outputs, and
    /// withdrawals. Helps reduce the state space for symbolic execution
    /// while preserving the properties being tested.
    function _narrowOperation(IPrivacyPool.Operation calldata op) public pure {
        for (uint256 i = 2; i < N_INPUTS; ++i) {
            vm.assume(op.nullifiers[i] == bytes32(uint256(0)));
            vm.assume(op.spendabilityAddresses[i] == address(0));
        }

        for (uint256 i = 2; i < N_OUTPUTS; ++i) {
            vm.assume(op.commitmentsOut[i] == bytes32(uint256(0)));
        }

        for (uint256 i = 2; i < N_WITHDRAWALS; ++i) {
            vm.assume(op.unshieldAmounts[i] == 0);
            vm.assume(op.unshieldAssets[i] == address(0));
            vm.assume(op.context.unshieldRecipients[i] == address(0));
        }
    }

    /// Checks the invariant that any operation that passes `verifyOperation`
    /// will also pass `_executeOperation`.
    ///
    /// SAFETY: See `_arbitraryState` and `_narrowOperation` for all assumptions.
    ///
    /// @dev This property is not safety-critical. It allows callers to use
    /// `verifyOperation` as a perfect pre-flight check and ensures calling
    /// `executePreVerified` on a verified operation will never revert.
    function check_verifiedOperationAlwaysExecutes(
        IPrivacyPool.Operation calldata op,
        uint128 consumed,
        uint128 staged,
        uint128 tip,
        uint128 oldRootIndex,
        bytes32 spentNullifier,
        bool spendabilityPasses
    ) public {
        _arbitraryState(
            op.oldRoot,
            consumed,
            staged,
            tip,
            oldRootIndex,
            spentNullifier,
            spendabilityPasses
        );

        _narrowOperation(op);

        // SAFETY: Unshield transfers are modelled as infallible.
        for (uint256 i; i < N_WITHDRAWALS; ++i) {
            vm.assume(op.unshieldAssets[i] == address(token));
        }

        tint.verifyOperation(op);
        try tint.executeOperation(op) {} catch {
            assert(false);
        }
    }

    /// Checks the invariant that the aggregation ring can always be drained.
    ///
    /// For any reachable state there must exist some operation that consumes every
    /// staged commitment, leaving `staged` at 0.
    ///
    /// @dev This acts as a liveness check. Tint can never get stuck in
    /// a state where it cannot be drained.
    function check_ringCanAlwaysBeDrained(
        bytes32 oldRoot,
        bytes32 newRoot,
        uint128 consumed,
        uint128 staged,
        uint128 tip,
        uint128 oldRootIndex
    ) public {
        _arbitraryState(
            oldRoot,
            consumed,
            staged,
            tip,
            oldRootIndex,
            bytes32(uint256(0)),
            true
        );

        IPrivacyPool.Operation memory op;
        op.oldRoot = oldRoot;
        op.newRoot = newRoot;
        op.startAggregationIndex = tint.consumed();
        op.endAggregationIndex = tint.consumed() + tint.staged();

        try tint.operate(op) {} catch {
            assert(false);
        }
        assert(tint.staged() == 0);
    }

    /// Checks the inverse invariant: while the ring has free space, more
    /// commitments can always be staged.
    ///
    /// For any reachable state with spare capacity there must exist some
    /// operation that stages an additional commitment.
    ///
    /// @dev The dual of `check_ringCanAlwaysBeDrained`. Tint can never get stuck
    /// in a state where it has room but refuses to accept new commitments.
    function check_ringCanAlwaysBeFilled(
        bytes32 oldRoot,
        bytes32 newRoot,
        bytes32 commitment,
        uint128 consumed,
        uint128 staged,
        uint128 tip,
        uint128 oldRootIndex
    ) public {
        _arbitraryState(
            oldRoot,
            consumed,
            staged,
            tip,
            oldRootIndex,
            bytes32(uint256(0)),
            true
        );

        // SAFETY: 0 is the magic "no commitment" value, which `_commit` skips.
        vm.assume(commitment != 0);

        // An operation that consumes nothing and stages a single output.
        IPrivacyPool.Operation memory op;
        op.oldRoot = oldRoot;
        op.newRoot = newRoot;
        op.startAggregationIndex = tint.consumed();
        op.endAggregationIndex = tint.consumed();
        op.commitmentsOut[0] = commitment;

        uint128 stagedBefore = tint.staged();
        try tint.operate(op) {} catch {
            assert(false);
        }
        assert(tint.staged() == stagedBefore + 1);
    }
}

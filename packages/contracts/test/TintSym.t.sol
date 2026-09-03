// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SymTest} from "halmos-cheatcodes/SymTest.sol";
import {Test} from "forge-std/Test.sol";
import {Tint} from "../src/Tint.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ISpendability} from "../src/interfaces/ISpendability.sol";
import {MockVerifier} from "../src/mocks/MockVerifier.sol";
import {LibCircularBuffer} from "../src/lib/LibCircularBuffer.sol";
import {LibCircularBufferInvariants} from "./LibCircularBufferSym.t.sol";
import {N_INPUTS, N_OUTPUTS, N_WITHDRAWALS, AGGREGATION_RING_SIZE} from "../src/lib/Constants.sol";

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
    constructor(IVerifier verifier) Tint(verifier) {}

    function setBuffer(LibCircularBuffer.CircularBuffer memory _buf) public {
        ring.buffer = _buf;
    }

    function setRoot(uint128 index, bytes32 root) public {
        ring.roots[index] = root;
    }

    function setSpent(bytes32 hash) public {
        isSpent[hash] = true;
    }

    function executeOperation(IPrivacyPool.Operation calldata op) public {
        _executeOperation(op);
    }

    function buffer() public view returns (LibCircularBuffer.CircularBuffer memory) {
        return ring.buffer;
    }

    /// @dev Override poseidon2 with cheaper keccak256. The specific hash function
    /// function is irrelevant.
    function _hash(bytes32 prevHash, bytes32 commitment) internal pure override returns (bytes32) {
        return keccak256(abi.encode(prevHash, commitment));
    }
}

contract TintSymTest is LibCircularBufferInvariants, SymTest {
    TintHarness tint;
    SymSpendability spendability;
    StubToken token;

    function setUp() public {
        token = new StubToken();
        spendability = new SymSpendability();
        tint = new TintHarness(new MockVerifier());
    }

    /// Produces an arbitrary reachable pool state.
    function _assumeState() internal {
        LibCircularBuffer.CircularBuffer memory buf;
        buf.head = uint128(svm.createUint(128, "head"));
        buf.tail = uint128(svm.createUint(128, "tail"));
        buf.buffer = new bytes32[](6);

        // SAFETY 001: Assumes circular buffer is valid.
        _assumeCircularBufferState(buf);
        tint.setBuffer(buf);

        // SAFETY 002: The root at the tail index must be non-zero.
        bytes32 tailRoot = svm.createBytes32("tailRootValue");
        vm.assume(tailRoot != bytes32(0));
        tint.setRoot(buf.tail, tailRoot);

        // STATE: Assumes a random root at a random index so the aggregation ring is not empty.
        bytes32 randomRoot = svm.createBytes32("randomRootValue");
        vm.assume(randomRoot != bytes32(0));
        tint.setRoot(uint128(svm.createUint(128, "randomRootIndex")), randomRoot);

        // STATE: Assume some arbitrary historical nullifier has been spent.
        bytes32 spentNullifier = svm.createBytes32("spentNullifier");
        vm.assume(spentNullifier != bytes32(0));
        tint.setSpent(spentNullifier);

        // STATE: Model spendability checks passing and failing.
        bool spendabilityPasses = svm.createBool("spendabilityPasses");
        spendability.setPass(spendabilityPasses);
    }

    /// Narrows the operation to have a max of two inputs, outputs, and
    /// withdrawals. Reduces the state space for symbolic execution.
    ///
    /// SAFETY: Narrowing to 1 field could introduce false positives because
    /// it eliminates interactions between inputs/output/withdrawals. 2 fields
    /// is sufficient to model all possible interactions.
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

    function _assertInvariants() internal view {
        // SAFETY 001
        _assertCircularBufferInvariants(tint.buffer());

        // SAFETY 002
        bytes32 tailRoot = tint.getRoot(tint.latestRootIndex());
        assert(tailRoot != bytes32(0));
    }

    /// Check that operate correctly advances the ring, records nullifiers, and maintains invariants.
    ///
    /// @dev Takes 15+ minutes to run.
    function check_operate(IPrivacyPool.Operation calldata op) public {
        _assumeState();
        _narrowOperation(op);

        uint128 latestRootIndexBefore = tint.latestRootIndex();
        bytes32 latestRootBefore = tint.getRoot(latestRootIndexBefore);

        tint.operate(op);

        if (op.endAggregationIndex > latestRootIndexBefore) {
            // Check that the operation advanced the ring correctly.
            assert(tint.latestRootIndex() == op.endAggregationIndex);
            assert(tint.getRoot(op.endAggregationIndex) == op.newRoot);
        } else {
            // Check that the operation did not advance the ring.
            assert(tint.latestRootIndex() == latestRootIndexBefore);
            assert(tint.getRoot(latestRootIndexBefore) == latestRootBefore);
        }

        for (uint256 i; i < N_INPUTS; ++i) {
            if (op.nullifiers[i] == bytes32(0)) continue;
            assert(tint.isSpent(op.nullifiers[i]));
        }

        _assertInvariants();
    }

    /// Checks the invariant that there is no operation for which `verifyOperation`
    /// succeeds but `executeOperation` reverts.
    ///
    /// @dev Assumes that all ERC20 transfers are infallible. In practice not true,
    /// but implementors could have erc20 whitelists to reduce risk.
    /// @dev Takes 5+ minutes to run.
    function check_verifiedOperationAlwaysExecutes(IPrivacyPool.Operation calldata op) public {
        _assumeState();
        _narrowOperation(op);

        // SAFETY: Unshield transfers are modelled as infallible.
        for (uint256 i; i < N_WITHDRAWALS; ++i) {
            vm.assume(op.unshieldAssets[i] == address(token));
        }

        tint.verifyOperation(op);
        try tint.executeOperation(op) {}
        catch {
            assert(false);
        }
    }

    /// Checks the invariant that the aggregation ring can always be drained.
    function check_ringCanAlwaysBeDrained(bytes32 newRoot) public {
        _assumeState();

        // SAFETY: `advance` always rejects the magic "no root" value.
        vm.assume(newRoot != bytes32(0));

        IPrivacyPool.Operation memory op;
        op.newRoot = newRoot;
        op.startAggregationIndex = tint.latestRootIndex();
        op.endAggregationIndex = tint.head();

        try tint.operate(op) {}
        catch {
            assert(false);
        }
        assert(tint.latestRootIndex() == tint.head());
        assert(tint.space() == 7);
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SymTest} from "halmos-cheatcodes/SymTest.sol";
import {Test} from "forge-std/Test.sol";
import {Tint} from "../src/Tint.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {ISpendability} from "../src/interfaces/ISpendability.sol";
import {MockVerifier} from "../src/mocks/MockVerifier.sol";
import {LibSkewMmrWithHistory} from "../src/lib/LibSkewMmrWithHistory.sol";
import {N_INPUTS, N_OUTPUTS, N_WITHDRAWALS} from "../src/lib/Constants.sol";

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
    using LibSkewMmrWithHistory for LibSkewMmrWithHistory.State;

    constructor(IVerifier verifier) Tint(verifier) {}

    /// @dev Appends without the ERC20 leg of `deposit`.
    function append(bytes32 commitment) public {
        mmr.append(commitment, _hash);
    }

    function setSpent(bytes32 hash) public {
        isSpent[hash] = true;
    }

    function executeOperation(IPrivacyPool.Operation calldata op) public {
        _executeOperation(op);
    }

    /// @dev Override poseidon2 with cheaper keccak256. The specific hash function
    /// is irrelevant.
    function _hash(bytes32 a, bytes32 b, bytes32 c) internal pure override returns (bytes32) {
        return keccak256(abi.encode(a, b, c));
    }
}

contract TintSymTest is Test, SymTest {
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
        for (uint256 i = 0; i < 4; ++i) {
            tint.append(svm.createBytes32(string(abi.encodePacked("commitment", i))));
        }

        // STATE: Assume some arbitrary historical nullifier has been spent.
        bytes32 spentNullifier = svm.createBytes32("spentNullifier");
        vm.assume(spentNullifier != bytes32(0));
        tint.setSpent(spentNullifier);

        // STATE: Model spendability checks passing and failing.
        bool spendabilityPasses = svm.createBool("spendabilityPasses");
        spendability.setPass(spendabilityPasses);
    }

    /// Binds the operation to the pool's live frontier.
    ///
    /// SAFETY: `verifyFrontier` rejects every other frontier, so leaving these
    /// symbolic would prune all paths through `operate` rather than explore them.
    /// Frontier authentication is covered by the skew-mmr package instead.
    function _bindFrontier(IPrivacyPool.Operation calldata op) public view {
        vm.assume(op.histState == tint.mmrState());

        bytes32[] memory live = tint.frontier();
        vm.assume(op.frontier.length == live.length);
        for (uint256 i = 0; i < live.length; ++i) {
            vm.assume(op.frontier[i] == live[i]);
        }
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

    /// Check that operate appends its outputs, records nullifiers, and maintains invariants.
    ///
    /// @custom:halmos --array-lengths op.frontier=2
    function check_operate(IPrivacyPool.Operation calldata op) public {
        _assumeState();
        _bindFrontier(op);
        _narrowOperation(op);

        uint256 countBefore = tint.mmrCount();

        tint.operate(op);

        // Every non-zero output commitment is appended; zero is the padding value.
        uint256 appended;
        for (uint256 i; i < N_OUTPUTS; ++i) {
            if (op.commitmentsOut[i] != bytes32(0)) ++appended;
        }
        assert(tint.mmrCount() == countBefore + appended);

        for (uint256 i; i < N_INPUTS; ++i) {
            if (op.nullifiers[i] == bytes32(0)) continue;
            assert(tint.isSpent(op.nullifiers[i]));
        }
    }

    /// Checks the invariant that there is no operation for which `verifyOperation`
    /// succeeds but `executeOperation` reverts.
    ///
    /// @dev Assumes that all ERC20 transfers are infallible. In practice not true,
    /// but implementors could have erc20 whitelists to reduce risk.
    ///
    /// @custom:halmos --array-lengths op.frontier=2
    function check_verifiedOperationAlwaysExecutes(IPrivacyPool.Operation calldata op) public {
        _assumeState();
        _bindFrontier(op);
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
}

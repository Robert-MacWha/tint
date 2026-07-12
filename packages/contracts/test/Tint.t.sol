// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {RootRegistry} from "../src/RootRegistry.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {N_INPUTS, N_OUTPUTS, N_PUB, GENESIS_ROOT} from "../src/lib/Constants.sol";

contract MockToken is ERC20 {
    constructor() ERC20("Mock", "MCK") {}

    function mint(address to, uint256 amount) public {
        super._mint(to, amount);
    }
}

contract MockVerifier {
    bool public shouldPass = true;

    function setPass(bool v) external {
        shouldPass = v;
    }

    function verifyProof(
        uint256[2] calldata,
        uint256[2][2] calldata,
        uint256[2] calldata,
        uint256[N_PUB] memory
    ) external view returns (bool) {
        return shouldPass;
    }
}

contract TintTests is Test {
    Tint public tint;
    MockToken public token;
    MockVerifier public verifier;

    bytes32 constant SEED = bytes32(uint256(0xdeadbeef));

    function setUp() public {
        token = new MockToken();
        verifier = new MockVerifier();
        tint = new Tint(address(verifier));
        token.mint(address(this), type(uint128).max);
        token.approve(address(tint), type(uint256).max);
        // Deposit one concrete commitment so leavesAggregationIndex=0 is valid in all operate tests.
        // Post-setUp: totalStaged=1, totalConsumed=0, roots[0]=1, currentRootIndex=1
        tint.deposit(address(token), 1, SEED, "");
    }

    // ------- helpers -------

    /// Builds a single-op array with sane defaults: oldRoot, nullifiers, newRoot=1, idx=0.
    /// Override fields directly on the returned array after calling.
    function _op(
        bytes32 oldRoot,
        bytes32[N_INPUTS] memory nullifiers
    ) internal pure returns (IPrivacyPool.Operation[] memory) {
        IPrivacyPool.Operation[] memory ops = new IPrivacyPool.Operation[](1);
        ops[0].oldRoot = oldRoot;
        ops[0].newRoot = bytes32(uint256(1));
        ops[0].leavesAggregationIndex = 0;
        ops[0].nullifiers = nullifiers;
        for (uint256 i; i < N_OUTPUTS; ++i) ops[0].spendabilityData[i] = "";
        return ops;
    }

    // ------- deposit() -------

    function test_deposit() public {
        uint256 tintBefore = token.balanceOf(address(tint));
        uint256 callerBefore = token.balanceOf(address(this));

        vm.expectEmit(true, true, true, true);
        emit Tint.Deposited(address(token), 100, bytes32(uint256(1)), "");
        tint.deposit(address(token), 100, bytes32(uint256(1)), "");

        assertEq(token.balanceOf(address(this)), callerBefore - 100);
        assertEq(token.balanceOf(address(tint)), tintBefore + 100);
    }

    function test_deposit_zeroAmount_reverts() public {
        vm.expectRevert(Tint.ZeroAmount.selector);
        tint.deposit(address(token), 0, SEED, "");
    }

    function test_deposit_zeroCommitment_reverts() public {
        vm.expectRevert(Tint.ZeroCommitment.selector);
        tint.deposit(address(token), 1, bytes32(0), "");
    }

    function test_deposit_noAllowance_reverts() public {
        token.approve(address(tint), 0);
        vm.expectRevert();
        tint.deposit(address(token), 1, bytes32(uint256(42)), "");
    }

    function test_deposit_insufficientBalance_reverts() public {
        MockToken fresh = new MockToken();
        fresh.mint(address(this), 1);
        fresh.approve(address(tint), type(uint256).max);
        vm.expectRevert();
        tint.deposit(address(fresh), 2, bytes32(uint256(42)), "");
    }

    // ------- operate() — validation -------

    function test_operate_invalidProof_reverts() public {
        verifier.setPass(false);
        bytes32[N_INPUTS] memory nullifiers;
        vm.expectRevert(Tint.InvalidProof.selector);
        tint.operate(_op(GENESIS_ROOT, nullifiers));
    }

    function test_operate_spentNullifier_reverts() public {
        bytes32[N_INPUTS] memory nullifiers;
        nullifiers[0] = bytes32("nullifier");
        tint.operate(_op(GENESIS_ROOT, nullifiers));

        tint.deposit(address(token), 1, bytes32(uint256(99)), "");
        IPrivacyPool.Operation[] memory ops = new IPrivacyPool.Operation[](1);
        ops[0].oldRoot = bytes32(uint256(1));
        ops[0].newRoot = bytes32(uint256(2));
        ops[0].leavesAggregationIndex = 1;
        ops[0].nullifiers = nullifiers;
        for (uint256 i; i < N_OUTPUTS; ++i) ops[0].spendabilityData[i] = "";
        vm.expectRevert(
            abi.encodeWithSelector(
                Tint.NullifierAlreadySpent.selector,
                bytes32("nullifier")
            )
        );
        tint.operate(ops);
    }

    function test_operate_unshieldZeroRecipient_reverts() public {
        bytes32[N_INPUTS] memory nullifiers;
        IPrivacyPool.Operation[] memory ops = _op(GENESIS_ROOT, nullifiers);
        ops[0].unshieldAmounts[0] = 1;
        ops[0].unshieldAssets[0] = address(token);
        // unshieldRecipients[0] stays address(0)
        vm.expectRevert(
            abi.encodeWithSelector(
                Tint.UnshieldRecipientZero.selector,
                uint256(0)
            )
        );
        tint.operate(ops);
    }

    // ------- operate() — state transitions -------

    function test_nullify() public {
        bytes32 nullifier = bytes32("nullifier");
        bytes32[N_INPUTS] memory nullifiers;
        nullifiers[0] = nullifier;
        vm.expectEmit(true, true, true, true);
        emit Tint.Nullified(nullifier);
        tint.operate(_op(GENESIS_ROOT, nullifiers));
        assertTrue(tint.nullifierHashes(nullifier));
    }

    function test_operate_marksAllNullifiers() public {
        bytes32[N_INPUTS] memory nullifiers;
        for (uint256 i; i < N_INPUTS; ++i)
            nullifiers[i] = bytes32(uint256(i + 1));
        tint.operate(_op(GENESIS_ROOT, nullifiers));
        for (uint256 i; i < N_INPUTS; ++i) {
            assertTrue(tint.nullifierHashes(bytes32(uint256(i + 1))));
        }
    }

    function test_operate_stagesOutputCommitments() public {
        bytes32[N_INPUTS] memory nullifiers;
        IPrivacyPool.Operation[] memory ops = _op(GENESIS_ROOT, nullifiers);
        ops[0].commitmentsOut[0] = bytes32(uint256(42));
        uint128 stagedBefore = tint.totalStaged();
        vm.expectEmit(true, true, true, true);
        emit Tint.Committed(bytes32(uint256(42)), address(0), "", "");
        tint.operate(ops);
        assertEq(tint.totalStaged(), stagedBefore + 1);
    }

    function test_operate_skipsZeroCommitments() public {
        bytes32[N_INPUTS] memory nullifiers;
        uint128 stagedBefore = tint.totalStaged();
        tint.operate(_op(GENESIS_ROOT, nullifiers)); // all commitmentsOut=0
        assertEq(tint.totalStaged(), stagedBefore);
    }

    function test_operate_unshield() public {
        // Contract holds 1 token from setUp's deposit
        bytes32[N_INPUTS] memory nullifiers;
        IPrivacyPool.Operation[] memory ops = _op(GENESIS_ROOT, nullifiers);
        ops[0].unshieldAmounts[0] = 1;
        ops[0].unshieldAssets[0] = address(token);
        ops[0].unshieldRecipients[0] = address(this);
        uint256 callerBefore = token.balanceOf(address(this));
        vm.expectEmit(true, true, true, true);
        emit Tint.Withdrawn(address(token), 1, address(this));
        tint.operate(ops);
        assertEq(token.balanceOf(address(this)), callerBefore + 1);
        assertEq(token.balanceOf(address(tint)), 0);
    }

    function test_operate_unshieldZeroAmountSkipped() public {
        bytes32[N_INPUTS] memory nullifiers;
        IPrivacyPool.Operation[] memory ops = _op(GENESIS_ROOT, nullifiers);
        ops[0].unshieldAmounts[0] = 0;
        ops[0].unshieldAssets[0] = address(token);
        uint256 tintBefore = token.balanceOf(address(tint));
        tint.operate(ops);
        assertEq(token.balanceOf(address(tint)), tintBefore); // no transfer
    }

    // ------- batch & integration -------

    function test_revert_on_nullifier_reuse() public {
        bytes32[N_INPUTS] memory nullifiers;
        nullifiers[0] = bytes32("nullifier");
        tint.operate(_op(GENESIS_ROOT, nullifiers));

        tint.deposit(address(token), 1, bytes32(uint256(99)), "");
        IPrivacyPool.Operation[] memory ops = new IPrivacyPool.Operation[](1);
        ops[0].oldRoot = bytes32(uint256(1));
        ops[0].newRoot = bytes32(uint256(2));
        ops[0].leavesAggregationIndex = 1;
        ops[0].nullifiers = nullifiers;
        for (uint256 i; i < N_OUTPUTS; ++i) ops[0].spendabilityData[i] = "";
        vm.expectRevert(
            abi.encodeWithSelector(
                Tint.NullifierAlreadySpent.selector,
                bytes32("nullifier")
            )
        );
        tint.operate(ops);
    }

    function test_operate_emptyBatch() public {
        IPrivacyPool.Operation[] memory ops = new IPrivacyPool.Operation[](0);
        tint.operate(ops);
    }

    function test_operate_batch_twoOps() public {
        tint.deposit(address(token), 1, bytes32(uint256(2)), ""); // totalStaged=2
        IPrivacyPool.Operation[] memory ops = new IPrivacyPool.Operation[](2);

        // Op0: 0→1 at idx=0
        ops[0].oldRoot = GENESIS_ROOT;
        ops[0].newRoot = bytes32(uint256(1));
        ops[0].leavesAggregationIndex = 0;
        for (uint256 i; i < N_OUTPUTS; ++i) ops[0].spendabilityData[i] = "";

        // Op1: 1→2 at idx=1 (roots[1] set by Op0 during the same call)
        ops[1].oldRoot = bytes32(uint256(1));
        ops[1].newRoot = bytes32(uint256(2));
        ops[1].leavesAggregationIndex = 1;
        for (uint256 i; i < N_OUTPUTS; ++i) ops[1].spendabilityData[i] = "";

        tint.operate(ops);
        assertEq(tint.currentRootIndex(), 3);
    }

    function test_operate_batch_secondOpFails_reverts() public {
        tint.deposit(address(token), 1, bytes32(uint256(2)), "");
        IPrivacyPool.Operation[] memory ops = new IPrivacyPool.Operation[](2);

        // Op0: valid, spends nullifier n1
        ops[0].oldRoot = GENESIS_ROOT;
        ops[0].newRoot = bytes32(uint256(1));
        ops[0].leavesAggregationIndex = 0;
        ops[0].nullifiers[0] = bytes32("n1");
        for (uint256 i; i < N_OUTPUTS; ++i) ops[0].spendabilityData[i] = "";

        // Op1: unknown root → whole tx reverts, rolling back Op0's state changes
        ops[1].oldRoot = bytes32(uint256(999));
        ops[1].newRoot = bytes32(uint256(2));
        ops[1].leavesAggregationIndex = 1;
        for (uint256 i; i < N_OUTPUTS; ++i) ops[1].spendabilityData[i] = "";

        vm.expectRevert(RootRegistry.InvalidOldRoot.selector);
        tint.operate(ops);

        assertFalse(tint.nullifierHashes(bytes32("n1"))); // rolled back
    }

    function test_depositThenOperate() public {
        tint.deposit(address(token), 1, bytes32(uint256(2)), ""); // idx=1 now valid
        bytes32[N_INPUTS] memory nullifiers;
        IPrivacyPool.Operation[] memory ops = _op(GENESIS_ROOT, nullifiers);
        ops[0].leavesAggregationIndex = 1;
        tint.operate(ops);
        assertEq(tint.totalConsumed(), 2); // advanced to idx+1=2
    }

    function test_rootChain() public {
        tint.deposit(address(token), 1, bytes32(uint256(2)), "");
        bytes32[N_INPUTS] memory nullifiers;
        bytes32 rootA = bytes32(uint256(1));
        bytes32 rootB = bytes32(uint256(2));

        tint.operate(_op(GENESIS_ROOT, nullifiers)); // 0→A, currentRootIndex=2
        assertEq(tint.roots(rootA), 2);

        IPrivacyPool.Operation[] memory ops = _op(rootA, nullifiers);
        ops[0].leavesAggregationIndex = 1;
        ops[0].newRoot = rootB;
        tint.operate(ops); // A→B, currentRootIndex=3
        assertEq(tint.roots(rootB), 3);
        assertEq(tint.currentRootIndex(), 3);
    }
}

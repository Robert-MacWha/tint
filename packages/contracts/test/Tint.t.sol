// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {RootRegistry} from "../src/RootRegistry.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {
    N_INPUTS,
    N_OUTPUTS,
    N_PUB,
    GENESIS_ROOT
} from "../src/lib/Constants.sol";

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
        // Deposit one concrete commitment so endAggregationIndex=1 is valid in all operate tests.
        // Post-setUp: totalStaged=1, totalConsumed=0, roots[0]=1, currentRootIndex=1
        tint.deposit(address(token), 1, SEED, "");
    }

    // ------- helpers -------

    /// Builds a single-op array with sane defaults: oldRoot, nullifiers, newRoot=1, idx=0.
    /// Override fields directly on the returned array after calling.
    function _op(
        bytes32 oldRoot,
        bytes32[N_INPUTS] memory nullifiers
    ) internal pure returns (IPrivacyPool.Operation memory) {
        IPrivacyPool.Operation memory op;
        op.oldRoot = oldRoot;
        op.newRoot = bytes32(uint256(1));
        op.endAggregationIndex = 0;
        op.nullifiers = nullifiers;
        for (uint256 i; i < N_INPUTS; ++i)
            op.spendabilityAddresses[i] = address(0);
        return op;
    }

    // ------- deposit() -------

    function test_deposit() public {
        uint256 tintBefore = token.balanceOf(address(tint));
        uint256 callerBefore = token.balanceOf(address(this));

        vm.expectEmit(true, true, true, true);
        emit Tint.Deposited(
            bytes32(
                0x276f9099e01965e0d0dc0cdfc20d83dea2dccc3e5470b8e0617acaccc5e6c5d5
            ),
            ""
        );
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
        IPrivacyPool.Operation memory op;
        op.oldRoot = bytes32(uint256(1));
        op.newRoot = bytes32(uint256(2));
        op.endAggregationIndex = 1;
        op.nullifiers = nullifiers;
        for (uint256 i; i < N_INPUTS; ++i)
            op.spendabilityAddresses[i] = address(0);
        vm.expectRevert(
            abi.encodeWithSelector(
                Tint.NullifierAlreadySpent.selector,
                bytes32("nullifier")
            )
        );
        tint.operate(op);
    }

    function test_operate_unshieldZeroRecipient_reverts() public {
        bytes32[N_INPUTS] memory nullifiers;
        IPrivacyPool.Operation memory op = _op(GENESIS_ROOT, nullifiers);
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        // unshieldRecipients[0] stays address(0)
        vm.expectRevert(
            abi.encodeWithSelector(
                Tint.UnshieldRecipientZero.selector,
                uint256(0)
            )
        );
        tint.operate(op);
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
        IPrivacyPool.Operation memory op = _op(GENESIS_ROOT, nullifiers);
        op.commitmentsOut[0] = bytes32(uint256(42));
        uint128 stagedBefore = tint.totalStaged();
        vm.expectEmit(true, true, true, true);
        emit Tint.Committed(bytes32(uint256(42)), "");
        tint.operate(op);
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
        IPrivacyPool.Operation memory op = _op(GENESIS_ROOT, nullifiers);
        op.unshieldAmounts[0] = 1;
        op.unshieldAssets[0] = address(token);
        op.unshieldRecipients[0] = address(this);
        uint256 callerBefore = token.balanceOf(address(this));
        vm.expectEmit(true, true, true, true);
        emit Tint.Withdrawn(address(token), 1, address(this));
        tint.operate(op);
        assertEq(token.balanceOf(address(this)), callerBefore + 1);
        assertEq(token.balanceOf(address(tint)), 0);
    }

    function test_operate_unshieldZeroAmountSkipped() public {
        bytes32[N_INPUTS] memory nullifiers;
        IPrivacyPool.Operation memory op = _op(GENESIS_ROOT, nullifiers);
        op.unshieldAmounts[0] = 0;
        op.unshieldAssets[0] = address(token);
        uint256 tintBefore = token.balanceOf(address(tint));
        tint.operate(op);
        assertEq(token.balanceOf(address(tint)), tintBefore); // no transfer
    }

    // ------- batch & integration -------

    function test_revert_on_nullifier_reuse() public {
        bytes32[N_INPUTS] memory nullifiers;
        nullifiers[0] = bytes32("nullifier");
        tint.operate(_op(GENESIS_ROOT, nullifiers));

        tint.deposit(address(token), 1, bytes32(uint256(99)), "");
        IPrivacyPool.Operation memory op = _op(GENESIS_ROOT, nullifiers);
        op.oldRoot = bytes32(uint256(1));
        op.newRoot = bytes32(uint256(2));
        op.endAggregationIndex = 1;
        op.nullifiers = nullifiers;
        for (uint256 i; i < N_INPUTS; ++i)
            op.spendabilityAddresses[i] = address(0);
        vm.expectRevert(
            abi.encodeWithSelector(
                Tint.NullifierAlreadySpent.selector,
                bytes32("nullifier")
            )
        );
        tint.operate(op);
    }

    function test_depositThenOperate() public {
        tint.deposit(address(token), 1, bytes32(uint256(2)), ""); // idx=1 now valid
        bytes32[N_INPUTS] memory nullifiers;
        IPrivacyPool.Operation memory op = _op(GENESIS_ROOT, nullifiers);
        op.endAggregationIndex = 1;
        tint.operate(op);
        assertEq(tint.totalConsumed(), 1);
    }

    function test_rootChain() public {
        tint.deposit(address(token), 1, bytes32(uint256(2)), "");
        bytes32[N_INPUTS] memory nullifiers;
        bytes32 rootA = bytes32(uint256(1));
        bytes32 rootB = bytes32(uint256(2));

        tint.operate(_op(GENESIS_ROOT, nullifiers)); // 0→A, currentRootIndex=2
        assertEq(tint.roots(rootA), 2);

        IPrivacyPool.Operation memory op = _op(rootA, nullifiers);
        op.endAggregationIndex = 1;
        op.newRoot = rootB;
        tint.operate(op); // A→B, currentRootIndex=3
        assertEq(tint.roots(rootB), 3);
        assertEq(tint.currentRootIndex(), 3);
    }
}

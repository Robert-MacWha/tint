// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {Tint} from "../src/Tint.sol";
import {TintFrame} from "../src/TintFrame.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPoseidon2T3} from "../src/interfaces/IPoseidon2T3.sol";
import {IWETH} from "../src/interfaces/IWETH.sol";
import {MockVerifier} from "./Tint.t.sol";
import {MockWETH} from "../src/mocks/MockWETH.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {IFrameOpcodes, TxParam, FrameParam, SigParam, FrameMode, SignatureScheme} from "../src/frame/IFrameOpcodes.sol";
import {MockFrameOpcodes} from "./mocks/MockFrameOpcodes.sol";
import {OperationLib} from "./OperationLib.sol";
import {N_INPUTS} from "../src/lib/Constants.sol";

contract TintFrameHarness is TintFrame {
    constructor(IVerifier _verifier, IPoseidon2T3 _poseidon, IWETH _weth, IFrameOpcodes _frame)
        TintFrame(_verifier, _poseidon, _weth, _frame)
    {}

    function requireNoPolicies(IPrivacyPool.Operation calldata op) external pure {
        _requireNoPolicies(op);
    }
}

contract TintFrameTest is Test {
    using OperationLib for IPrivacyPool.Operation;

    /// @dev Where unpatched builds of `TintFrame._approve` record the APPROVE.
    bytes32 constant APPROVE_MARKER = 0x7f40b94fdfe5e13e90a14e3049fcb105a7f986f289ec0e6751cc92850ad1121b;
    string constant UNPATCHED_INITCODE = "out/frame/TintFrame.unpatched.hex";

    uint128 constant FEE = 1 ether;
    uint128 constant UNSHIELD = 5;
    address constant AUTHORIZER = address(0xa11ce);
    address constant RECIPIENT = address(0xdead);

    MockVerifier verifier;
    IPoseidon2T3 poseidon;
    MockWETH weth;
    MockFrameOpcodes frame;
    TintFrame tint;
    IPrivacyPool.Operation op;

    function setUp() public {
        verifier = new MockVerifier();
        poseidon = new Poseidon2T3();
        weth = new MockWETH();
        frame = new MockFrameOpcodes();
        _use(new TintFrame(verifier, poseidon, weth, frame));
    }

    /// -------------------- validate() --------------------

    function test_validate_approves() public {
        tint.validate();
        assertEq(vm.load(address(tint), APPROVE_MARKER), bytes32(uint256(3)));
    }

    function test_validateNotFirstFrame_reverts() public {
        frame.setTxParam(TxParam.CURRENT_FRAME, 1);
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    function test_validateExecuteNotSender_reverts() public {
        frame.setFrameParam(1, FrameParam.MODE, FrameMode.DEFAULT);
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    function test_validateExecuteOtherTarget_reverts() public {
        frame.setFrameParam(1, FrameParam.TARGET, uint160(address(0xbad)));
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    /// An atomic batch would let a later frame's failure roll back the execution
    /// after the float has paid.
    function test_validateExecuteAtomic_reverts() public {
        frame.setFrameParam(1, FrameParam.FLAGS, 4);
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    function test_validateExecuteWithValue_reverts() public {
        frame.setFrameParam(1, FrameParam.VALUE, 1);
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    function test_validateExecuteLowGas_reverts() public {
        frame.setFrameParam(1, FrameParam.GAS_LIMIT, tint.MIN_EXECUTE_GAS() - 1);
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    function test_validateExecuteLowStateGas_reverts() public {
        frame.setFrameParam(1, FrameParam.STATE_GAS_LIMIT, tint.MIN_EXECUTE_STATE_GAS() - 1);
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    /// A second SENDER frame could call anything as the pool, e.g. `WETH.transfer`.
    function test_validateLaterSenderFrame_reverts() public {
        frame.setFrameParam(2, FrameParam.MODE, FrameMode.SENDER);
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    function test_validateLaterVerifyFrame() public {
        frame.setFrameParam(2, FrameParam.MODE, FrameMode.VERIFY);
        tint.validate();
        assertEq(vm.load(address(tint), APPROVE_MARKER), bytes32(uint256(3)));
    }

    function test_validateWrongSelector_reverts() public {
        frame.setFrameData(1, abi.encodeCall(Tint.operate, (op)));
        _expectValidateRevert(TintFrame.InvalidFrames.selector);
    }

    /// -------------------- _requireNoPolicies() --------------------

    function test_requireNoPolicies() public {
        _harness().requireNoPolicies(op);
    }

    function test_requireNoPoliciesPolicyAddress_reverts(uint256 i) public {
        IPrivacyPool.Operation memory o = op;
        o.setPolicyAddress(bound(i, 0, N_INPUTS - 1), address(0xb0b));
        TintFrameHarness harness = _harness();
        vm.expectRevert(TintFrame.PolicyNotSupported.selector);
        harness.requireNoPolicies(o);
    }

    function test_requireNoPoliciesPolicyInput_reverts(uint256 i) public {
        IPrivacyPool.Operation memory o = op;
        o.context.policyInputs[bound(i, 0, N_INPUTS - 1)] = hex"01";
        TintFrameHarness harness = _harness();
        vm.expectRevert(TintFrame.PolicyNotSupported.selector);
        harness.requireNoPolicies(o);
    }

    function test_validatePolicy_reverts() public {
        IPrivacyPool.Operation memory o = op;
        o.setPolicyAddress(0, address(0xb0b));
        frame.setFrameData(1, abi.encodeCall(TintFrame.executeSponsored, (o.seal())));
        _expectValidateRevert(TintFrame.PolicyNotSupported.selector);
    }

    function test_validatePolicyInput_reverts() public {
        IPrivacyPool.Operation memory o = op;
        o.context.policyInputs[2] = hex"01";
        frame.setFrameData(1, abi.encodeCall(TintFrame.executeSponsored, (o.seal())));
        _expectValidateRevert(TintFrame.PolicyNotSupported.selector);
    }

    function test_validateInvalidProof_reverts() public {
        verifier.setPass(false);
        _expectValidateRevert(Tint.InvalidProof.selector);
    }

    function test_validateUnbound_reverts() public {
        IPrivacyPool.Operation memory o = op;
        o.context.binding = bytes32(0);
        frame.setSigParam(0, SigParam.SIGNER, 0);
        frame.setFrameData(1, abi.encodeCall(TintFrame.executeSponsored, (o.seal())));
        _expectValidateRevert(TintFrame.InvalidBinding.selector);
    }

    function test_validateWrongSigner_reverts() public {
        frame.setSigParam(0, SigParam.SIGNER, uint160(address(0xb0b)));
        _expectValidateRevert(TintFrame.InvalidBinding.selector);
    }

    function test_validateWrongScheme_reverts() public {
        frame.setSigParam(0, SigParam.SCHEME, SignatureScheme.P256);
        _expectValidateRevert(TintFrame.InvalidBinding.selector);
    }

    /// A signature over its own `msg` doesn't cover the transaction's frames.
    function test_validateSignatureWithMsg_reverts() public {
        frame.setSigParam(0, SigParam.MSG, 1);
        _expectValidateRevert(TintFrame.InvalidBinding.selector);
    }

    function test_validateInsufficientFee_reverts() public {
        frame.setTxParam(TxParam.MAX_COST, FEE + 1);
        _expectValidateRevert(TintFrame.InsufficientFee.selector);
    }

    /// Only WETH paid to the pool counts towards the fee.
    function test_validateFeeToOtherRecipient_reverts() public {
        IPrivacyPool.Operation memory o = op;
        o.setUnshieldRecipient(0, RECIPIENT);
        frame.setFrameData(1, abi.encodeCall(TintFrame.executeSponsored, (o.seal())));
        _expectValidateRevert(TintFrame.InsufficientFee.selector);
    }

    /// -------------------- executeSponsored() --------------------

    function test_executeSponsored() public {
        _executeSponsored();
    }

    function test_executeSponsoredNotSelf_reverts() public {
        vm.expectRevert(TintFrame.OnlySelf.selector);
        tint.executeSponsored(op);
    }

    /// -------------------- IR round trip --------------------

    /// The deployed TintFrame is assembled from its Yul IR by `just build-frame` rather
    /// than by forge, so run the happy path against that pipeline's unpatched output.
    function test_irRoundTrip() public {
        if (!vm.exists(UNPATCHED_INITCODE)) {
            vm.skip(true, "run `just build-frame` first");
        }

        bytes memory initcode =
            bytes.concat(vm.parseBytes(vm.readFile(UNPATCHED_INITCODE)), abi.encode(verifier, poseidon, weth, frame));
        TintFrame deployed;
        assembly {
            deployed := create(0, add(initcode, 0x20), mload(initcode))
        }
        assertTrue(address(deployed).code.length > 0);
        _use(deployed);

        tint.validate();
        assertEq(vm.load(address(tint), APPROVE_MARKER), bytes32(uint256(3)));
        _executeSponsored();
    }

    /// -------------------- helpers --------------------

    /// Points the tests at `t`: a deposit to spend, and a well-formed sponsored transaction
    /// for an operation that pays `FEE` and unshields `UNSHIELD` to `RECIPIENT`.
    function _use(TintFrame t) internal {
        tint = t;

        vm.deal(address(this), FEE + UNSHIELD);
        weth.deposit{value: FEE + UNSHIELD}();
        weth.approve(address(tint), FEE + UNSHIELD);
        tint.deposit(address(weth), FEE + UNSHIELD, bytes32(uint256(1)), "");

        IPrivacyPool.Operation memory o = OperationLib.bind(tint);
        o.setNullifier(0, bytes32(uint256(123)));
        o.setCommitmentOut(0, bytes32(uint256(42)));
        o.setUnshieldAmount(0, FEE);
        o.setUnshieldAsset(0, address(weth));
        o.setUnshieldRecipient(0, address(tint));
        o.setUnshieldAmount(1, UNSHIELD);
        o.setUnshieldAsset(1, address(weth));
        o.setUnshieldRecipient(1, RECIPIENT);
        o.context.binding = bytes32(uint256(uint160(AUTHORIZER)));
        op = o.seal();

        frame.setTxParam(TxParam.CURRENT_FRAME, 0);
        frame.setTxParam(TxParam.FRAME_COUNT, 3);
        frame.setTxParam(TxParam.MAX_COST, FEE);

        frame.setFrameParam(1, FrameParam.MODE, FrameMode.SENDER);
        frame.setFrameParam(1, FrameParam.TARGET, uint160(address(tint)));
        frame.setFrameParam(1, FrameParam.FLAGS, 0);
        frame.setFrameParam(1, FrameParam.VALUE, 0);
        frame.setFrameParam(1, FrameParam.GAS_LIMIT, tint.MIN_EXECUTE_GAS());
        frame.setFrameParam(1, FrameParam.STATE_GAS_LIMIT, tint.MIN_EXECUTE_STATE_GAS());
        frame.setFrameData(1, abi.encodeCall(TintFrame.executeSponsored, (op)));
        frame.setFrameParam(2, FrameParam.MODE, FrameMode.DEFAULT);

        frame.setSigParam(0, SigParam.SIGNER, uint160(AUTHORIZER));
        frame.setSigParam(0, SigParam.SCHEME, SignatureScheme.SECP256K1);
        frame.setSigParam(0, SigParam.MSG, 0);
    }

    function _harness() internal returns (TintFrameHarness) {
        return new TintFrameHarness(verifier, poseidon, weth, frame);
    }

    function _expectValidateRevert(bytes4 selector) internal {
        vm.expectRevert(selector);
        tint.validate();
    }

    /// Runs frame 1: spends the note, unshields, and unwraps the fee into the float.
    function _executeSponsored() internal {
        uint256 floatBefore = address(tint).balance;

        vm.prank(address(tint));
        tint.executeSponsored(op);

        assertTrue(tint.isSpent(bytes32(uint256(123))));
        assertEq(address(tint).balance, floatBefore + FEE);
        assertEq(weth.balanceOf(RECIPIENT), UNSHIELD);
        assertEq(weth.balanceOf(address(tint)), 0);
    }
}

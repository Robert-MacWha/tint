// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Tint} from "../src/Tint.sol";
import {TintVerifier} from "../src/TintVerifier.sol";
import {Poseidon2T3} from "../src/poseidon/Poseidon2T3.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";
import {IPrivacyPool} from "../src/interfaces/IPrivacyPool.sol";
import {OperationLib} from "../test/OperationLib.sol";
import {N_COMPRESSED_PUB} from "../src/lib/Constants.sol";

contract BenchToken is ERC20 {
    constructor() ERC20("Bench", "BNC") {
        _mint(msg.sender, type(uint128).max);
    }
}

/// @notice Runs the real pairing check but always accepts.
contract AcceptingVerifier is IVerifier {
    TintVerifier public immutable INNER = new TintVerifier();

    function verify(uint256[8] calldata proof, uint256[N_COMPRESSED_PUB] calldata pubSignals) external view {
        try INNER.verify(proof, pubSignals) {} catch {}
    }
}

/// @notice Calldata for deposits and operations carrying ciphertexts of each
/// candidate note-encryption layout, to measure real tx gas from receipts.
///
/// `run` deploys a pool to a local anvil; `cases(tint, token)` logs each
/// case's label and calldata. Send each from the same `evm_snapshot` and
/// read `gasUsed` from its receipt.
contract NoteEncryptionGas is Script {
    using OperationLib for IPrivacyPool.Operation;

    uint256 constant TAG = 1;
    uint256 constant XWING = 1120;
    uint256 constant MLKEM512_X25519 = 800 + 32;
    uint256 constant NOTE = 136 + 16;
    uint256 constant PARTIAL = 99 + 16;
    uint256 constant X25519_NOTE = 260;
    uint256 constant X25519_PARTIAL = 223;

    Tint tint;
    BenchToken token;
    uint256 nonce;

    function run() external {
        vm.startBroadcast();
        token = new BenchToken();
        tint = new Tint(new AcceptingVerifier(), new Poseidon2T3());
        token.approve(address(tint), type(uint256).max);
        token.transfer(address(tint), 1_000_000);
        for (uint256 i; i < 6; ++i) {
            tint.deposit(address(token), 1, bytes32(i + 1), "");
        }
        vm.stopBroadcast();
        console.log("tint", address(tint));
        console.log("token", address(token));
    }

    function cases(Tint _tint, BenchToken _token) external {
        tint = _tint;
        token = _token;

        _deposit("empty", 0);
        _deposit("x25519", X25519_PARTIAL);
        _deposit("per-note xwing", TAG + XWING + PARTIAL);
        _deposit("per-note mlkem512", TAG + MLKEM512_X25519 + PARTIAL);

        uint256[3] memory inputs = [uint256(1), 1, 5];
        uint256[3] memory outputs = [uint256(1), 2, 5];
        for (uint256 s; s < 3; ++s) {
            uint256 i = inputs[s];
            uint256 o = outputs[s];
            _operate("empty", i, o, 0, 0);
            _operate("x25519", i, o, 0, X25519_NOTE);
            _operate("per-note xwing", i, o, 0, TAG + XWING + NOTE);
            _operate("per-note mlkem512", i, o, 0, TAG + MLKEM512_X25519 + NOTE);
            _operate("per-op xwing", i, o, TAG + XWING, NOTE);
            _operate("per-op mlkem512", i, o, TAG + MLKEM512_X25519, NOTE);
        }
    }

    function _deposit(string memory label, uint256 len) internal {
        bytes memory data = abi.encodeCall(Tint.deposit, (address(token), 1, bytes32(++nonce), _random(len)));
        console.log(string.concat("deposit ", label, " ", vm.toString(len), " ", vm.toString(data)));
    }

    /// @dev `head` extra bytes ride on the first output, every output carries `each`.
    function _operate(string memory label, uint256 nInputs, uint256 nOutputs, uint256 head, uint256 each) internal {
        IPrivacyPool.Operation memory op = OperationLib.bind(tint);
        for (uint256 i; i < nInputs; ++i) {
            op.setNullifier(i, bytes32(++nonce));
        }
        uint256 total;
        for (uint256 i; i < nOutputs; ++i) {
            op.setCommitmentOut(i, bytes32(++nonce));
            uint256 len = each + (i == 0 ? head : 0);
            op.context.ciphertexts[i] = _random(len);
            total += len;
        }
        bytes memory data = abi.encodeCall(Tint.operate, (op.seal()));
        string memory shape = string.concat(vm.toString(nInputs), "x", vm.toString(nOutputs));
        console.log(string.concat("operate-", shape, " ", label, " ", vm.toString(total), " ", vm.toString(data)));
    }

    /// @dev Pseudorandom bytes, so calldata is ~all non-zero like real ciphertext.
    function _random(uint256 len) internal returns (bytes memory out) {
        out = new bytes(len);
        for (uint256 i; i < len; i += 32) {
            bytes32 word = keccak256(abi.encode(++nonce));
            for (uint256 j; j < 32 && i + j < len; ++j) {
                out[i + j] = word[j];
            }
        }
    }
}

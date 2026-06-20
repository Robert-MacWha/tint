// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

// Run with:
//   docker run -v .:/workspace ghcr.io/a16z/halmos:latest \
//     halmos --contract TintFormal --forge-build-out packages/contracts/out --loop 6

import {Test} from "forge-std/Test.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Tint} from "../src/Tint.sol";
import {IVerifier} from "../src/IVerifier.sol";
import {IArchiveVerifier} from "../src/IArchiveVerifier.sol";

/// Verifier stub that unconditionally accepts every proof.
/// This lets us treat proof validity as an axiom and focus
/// on the contract's own invariants.
contract TrustedVerifier is IVerifier {
    function verifyProof(uint[2] memory, uint[2][2] memory, uint[2] memory, uint[44] memory)
        external pure returns (bool) { return true; }
}

contract TrustedArchiveVerifier is IArchiveVerifier {
    function verifyProof(uint[2] memory, uint[2][2] memory, uint[2] memory, uint[3] memory)
        external pure returns (bool) { return true; }
}

/// Minimal ERC-20 stub for properties that need shield() to succeed.
contract MockToken is IERC20 {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function mint(address to, uint256 amount) external { balanceOf[to] += amount; }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }
    function transfer(address to, uint256 amount) external returns (bool) {
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }
    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
    function totalSupply() external pure returns (uint256) { return type(uint256).max; }
}

contract TintFormal is Test {
    Tint tint;
    MockToken token;

    function setUp() public {
        tint = new Tint(address(new TrustedVerifier()), address(new TrustedArchiveVerifier()));
        token = new MockToken();
        token.mint(address(this), type(uint256).max);
        token.approve(address(tint), type(uint256).max);
    }

    // Encode a transact call with symbolic nullifiers and zeroed outputs.
    // Zero outputs mean the only revert path inside transact is the nullifier check,
    // isolating the property under test from LeanIMT and token-transfer side effects.
    function _transactCall(uint256[6] memory nullifiers) internal view returns (bytes memory) {
        uint256[2] memory pA;
        uint256[2][2] memory pB;
        uint256[2] memory pC;
        uint256[6] memory zeros;
        address[6] memory zeroAddrs;
        bytes[6] memory emptyData;
        return abi.encodeCall(
            tint.transact,
            (pA, pB, pC, nullifiers, zeros, zeros, zeros, zeros, zeroAddrs, emptyData)
        );
    }

    function _assumeDistinctUnspent(uint256[6] memory nullifiers) internal view {
        for (uint256 i; i < 6; ++i) {
            vm.assume(nullifiers[i] != 0); // exclude dummy sentinel
            vm.assume(!tint.spent(nullifiers[i]));
            for (uint256 j = i + 1; j < 6; ++j) {
                vm.assume(nullifiers[i] != nullifiers[j]);
            }
        }
    }

    /// Safety: a nullifier can never be spent more than once.
    ///
    /// Given any six distinct, unspent nullifiers, if the first transact call
    /// succeeds then an identical second call must revert.
    function check_doubleSpend(uint256[6] memory nullifiers) public {
        _assumeDistinctUnspent(nullifiers);

        bytes memory cd = _transactCall(nullifiers);

        (bool first,) = address(tint).call(cd);
        vm.assume(first);

        (bool second,) = address(tint).call(cd);
        assert(!second);
    }

    /// Liveness: a fresh nullifier can always be spent.
    ///
    /// Given any six distinct, unspent nullifiers and zero outputs (no LeanIMT
    /// insertions, no token transfers), the transact call must succeed and mark
    /// all six nullifiers as spent.
    function check_freshNullifiersAlwaysSpendable(uint256[6] memory nullifiers) public {
        _assumeDistinctUnspent(nullifiers);

        (bool ok,) = address(tint).call(_transactCall(nullifiers));
        assert(ok);

        for (uint256 i; i < 6; ++i) {
            assert(tint.spent(nullifiers[i]));
        }
    }

    /// Invariant: spent[n] is permanent — no function can flip it from true back to false.
    ///
    /// After spending nullifiers in one transact, a subsequent transact (with any
    /// other inputs) cannot un-spend them.
    function check_spentIsPermanent(
        uint256[6] memory nullifiers,
        uint256[6] memory otherNullifiers
    ) public {
        _assumeDistinctUnspent(nullifiers);

        (bool first,) = address(tint).call(_transactCall(nullifiers));
        vm.assume(first);

        // Attempt any second transact (may succeed or revert — we don't care).
        address(tint).call(_transactCall(otherNullifiers));

        // Original nullifiers must still be spent.
        for (uint256 i; i < 6; ++i) {
            assert(tint.spent(nullifiers[i]));
        }
    }

    /// Isolation: shield() never affects the nullifier set.
    ///
    /// A shield call cannot mark any nullifier as spent, regardless of inputs.
    function check_shieldDoesNotAffectSpent(
        uint256 nullifier,
        uint256 amount,
        uint256 commitment
    ) public {
        vm.assume(!tint.spent(nullifier));
        vm.assume(commitment != 0); // LeanIMT rejects zero leaves

        address(tint).call(
            abi.encodeCall(tint.shield, (address(token), amount, commitment))
        );

        assert(!tint.spent(nullifier));
    }

    /// Commitment uniqueness: the same commitment cannot enter the staging tree twice.
    ///
    /// If shield succeeds with commitment C, any subsequent shield with the same C
    /// must revert (LeanIMT LeafAlreadyExists).
    function check_duplicateCommitmentReverts(uint256 commitment, uint256 amount) public {
        vm.assume(commitment != 0);
        vm.assume(amount > 0);

        (bool first,) = address(tint).call(
            abi.encodeCall(tint.shield, (address(token), amount, commitment))
        );
        vm.assume(first);

        (bool second,) = address(tint).call(
            abi.encodeCall(tint.shield, (address(token), amount, commitment))
        );
        assert(!second);
    }
}

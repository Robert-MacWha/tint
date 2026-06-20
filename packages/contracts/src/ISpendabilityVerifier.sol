// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

// Interface for snarkjs-generated spendability circuit verifiers.
// Public signals: [nullifier, commitmentsOut[0..5]] — 7 total.
interface ISpendabilityVerifier {
    function verifyProof(
        uint[2] memory pA,
        uint[2][2] memory pB,
        uint[2] memory pC,
        uint[7] memory pubSignals
    ) external view returns (bool);
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

// Proves newArchiveRoot = LeanIMT.insert(oldArchiveRoot, stagingRoot).
// Public signals: [oldArchiveRoot, stagingRoot, newArchiveRoot]
interface IArchiveVerifier {
    function verifyProof(
        uint[2] memory pA,
        uint[2][2] memory pB,
        uint[2] memory pC,
        uint[3] memory pubSignals
    ) external view returns (bool);
}

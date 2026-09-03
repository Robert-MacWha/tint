// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IVerifier} from "./interfaces/IVerifier.sol";
import {Verifier} from "./codegen/TintVerifier.sol";
import {N_PUB} from "./lib/Constants.sol";

contract TintVerifier is IVerifier, Verifier {
    function verify(uint256[8] calldata proof, uint256[N_PUB] calldata pubSignals) external view {
        super.verifyProof(proof, pubSignals);
    }
}

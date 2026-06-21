pragma circom 2.2.3;

include "../node_modules/circomlib/circuits/eddsaposeidon.circom";

/// Basic spendability circuit that verifies multiple EdDSA Poseidon signatures.
template EddsaPoseidonMultisigSpendability(nOutputs, nSignatures) {
    /// Public signals
    signal input nullifier;                // Nullifier of the note being spent.
    signal input commitmentsOut[nOutputs]; // Commitments for output notes in this operation.
    signal input boundParamsHash;          // Hash of bound params for this circuit.

    /// Private signals
    signal input pubKeyAx[nSignatures];
    signal input pubKeyAy[nSignatures];
    signal input signatureS[nSignatures];
    signal input signatureR8x[nSignatures];
    signal input signatureR8y[nSignatures];

    /// Verify the bound parameters hash
    component computeBoundParamsHash = Poseidon(nSignatures * 2);
    for (var i = 0; i < nSignatures; i++) {
        computeBoundParamsHash.inputs[i * 2] <== pubKeyAx[i];
        computeBoundParamsHash.inputs[i * 2 + 1] <== pubKeyAy[i];
    }
    computeBoundParamsHash.out === boundParamsHash;

    // Verify the signature
    component eddsaVerifiers[nSignatures];
    for (var i = 0; i < nSignatures; i++) {
        eddsaVerifiers[i] = EdDSAPoseidonVerifier();
        eddsaVerifiers[i].enabled <== 1;
        eddsaVerifiers[i].Ax <== pubKeyAx[i];
        eddsaVerifiers[i].Ay <== pubKeyAy[i];
        eddsaVerifiers[i].S <== signatureS[i];
        eddsaVerifiers[i].R8x <== signatureR8x[i];
        eddsaVerifiers[i].R8y <== signatureR8y[i];
        eddsaVerifiers[i].M <== nullifier;
    }
}

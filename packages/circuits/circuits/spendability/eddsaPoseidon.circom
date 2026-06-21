pragma circom 2.2.3;

include "../node_modules/circomlib/circuits/eddsaposeidon.circom";

/// Basic spendability circuit that verifies an EdDSA Poseidon signature.
template EddsaPoseidonSpendability(nOutputs) {
    /// Public signals
    signal input nullifier;                // Nullifier of the note being spent.
    signal input commitmentsOut[nOutputs]; // Commitments for output notes in this operation.
    signal input boundParamsHash;          // Hash of bound params for this circuit.

    /// Private signals
    signal input pubKeyAx;
    signal input pubKeyAy;
    signal input signatureS;
    signal input signatureR8x;
    signal input signatureR8y;

    // Verify that the boundParamsHash matches the pubkey
    component hashPubKey = Poseidon(2);
    hashPubKey.inputs[0] <== pubKeyAx;
    hashPubKey.inputs[1] <== pubKeyAy;
    hashPubKey.out === boundParamsHash;

    // Verify the signature
    component eddsaVerifier = EdDSAPoseidonVerifier();
    eddsaVerifier.enabled <== 1;
    eddsaVerifier.Ax <== pubKeyAx;
    eddsaVerifier.Ay <== pubKeyAy;
    eddsaVerifier.S <== signatureS;
    eddsaVerifier.R8x <== signatureR8x;
    eddsaVerifier.R8y <== signatureR8y;
    eddsaVerifier.M <== nullifier;
}

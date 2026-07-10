pragma circom 2.2.3;

include "../node_modules/circomlib/circuits/poseidon.circom";

template CommitmentHasher() {
    signal input asset;
    signal input amount;
    signal input partialCommitment;

    signal output commitment;

    component hasher = Poseidon(3);
    hasher.inputs[0] <== asset;
    hasher.inputs[1] <== amount;
    hasher.inputs[2] <== partialCommitment;

    commitment <== hasher.out;
}

/// Computes poseidon(spendabilityHash, nullifyingPubKey, random).
/// Binds spendability data and the nullifying public key to the commitment,
/// with random ensuring uniqueness across notes with the same key/asset/amount.
template PartialCommitmentHasher() {
    signal input spendabilityHash;
    signal input nullifyingPubKey;
    signal input random;

    signal output partialCommitment;

    component hasher = Poseidon(3);
    hasher.inputs[0] <== spendabilityHash;
    hasher.inputs[1] <== nullifyingPubKey;
    hasher.inputs[2] <== random;

    partialCommitment <== hasher.out;
}

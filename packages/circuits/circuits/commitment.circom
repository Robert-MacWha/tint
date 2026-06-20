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

template PartialCommitmentHasher() {
    signal input random;
    signal input nullifyingPubKey;
    signal input spendabilityAddress;
    signal input spendabilityData;

    signal output partialCommitment;

    component hasher = Poseidon(4);
    hasher.inputs[0] <== random;
    hasher.inputs[1] <== nullifyingPubKey;
    hasher.inputs[2] <== spendabilityAddress;
    hasher.inputs[3] <== spendabilityData;

    partialCommitment <== hasher.out;
}
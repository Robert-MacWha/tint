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

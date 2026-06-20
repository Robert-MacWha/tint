pragma circom 2.2.3;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/bitify.circom";
include "../node_modules/circomlib/circuits/mux1.circom";

/**
 * Builds a complete binary Merkle tree from 2^depth leaves and outputs the root.
 * Leaves at unused slots should be 0. All internal nodes are Poseidon(left, right).
 */
template BatchMerkleRoot(depth) {
    var nLeaves = 1 << depth;

    signal input leaves[nLeaves];
    signal output root;

    // Flat storage: nodes[0..nLeaves-1] = leaves, then levels above packed consecutively.
    // Total nodes in a complete binary tree = 2*nLeaves - 1.
    signal nodes[2 * nLeaves - 1];
    component poseidons[nLeaves - 1];

    for (var i = 0; i < nLeaves; i++) {
        nodes[i] <== leaves[i];
    }

    var levelStart = 0;
    var levelSize = nLeaves;
    var hashIdx = 0;

    while (levelSize > 1) {
        for (var j = 0; j < levelSize / 2; j++) {
            poseidons[hashIdx] = Poseidon(2);
            poseidons[hashIdx].inputs[0] <== nodes[levelStart + 2 * j];
            poseidons[hashIdx].inputs[1] <== nodes[levelStart + 2 * j + 1];
            nodes[levelStart + levelSize + j] <== poseidons[hashIdx].out;
            hashIdx++;
        }
        levelStart = levelStart + levelSize;
        levelSize = levelSize / 2;
    }

    root <== nodes[2 * nLeaves - 2];
}

/**
 * Verifies that a leaf is included in a complete binary Merkle tree of depth `depth`.
 * Unlike LeanIMTProofVerifier, does not promote single-child nodes — all sibling slots
 * are always populated (possibly with zero for unused leaf slots).
 */
template BatchMerkleInclusion(depth) {
    signal input leaf;
    signal input leafIndex;
    signal input siblings[depth];

    signal output root;

    signal nodes[depth + 1];
    signal indices[depth];

    component indexToPath = Num2Bits(depth);
    indexToPath.in <== leafIndex;
    indices <== indexToPath.out;

    nodes[0] <== leaf;

    component hashInCorrectOrder[depth];
    component poseidons[depth];

    for (var i = 0; i < depth; i++) {
        var childrenToSort[2][2] = [[nodes[i], siblings[i]], [siblings[i], nodes[i]]];
        hashInCorrectOrder[i] = MultiMux1(2);
        hashInCorrectOrder[i].c <== childrenToSort;
        hashInCorrectOrder[i].s <== indices[i];

        poseidons[i] = Poseidon(2);
        poseidons[i].inputs <== hashInCorrectOrder[i].out;

        nodes[i + 1] <== poseidons[i].out;
    }

    root <== nodes[depth];
}

pragma circom 2.2.0;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/mux1.circom";
include "./comparators.circom";

/// Computes a standard fixed-depth IMT root from its frontier and tree size.
///
/// zeros[l] = Poseidon(zeros[l-1], zeros[l-1]) is the hash of an empty 2^l-leaf subtree.
/// For each level, if bit[l]=1 the frontier node is the left child; otherwise the current
/// node is left and zeros[l] is right.  Simpler than LeanIMT's variable-depth state machine.
template MerkleTreeRootFromFrontier(MAX_DEPTH) {
    signal input frontier[MAX_DEPTH];
    signal input size;
    signal output out;

    component zeroHashers[MAX_DEPTH];
    signal zeros[MAX_DEPTH + 1];
    zeros[0] <== 0;
    for (var l = 0; l < MAX_DEPTH; l++) {
        zeroHashers[l] = Poseidon(2);
        zeroHashers[l].inputs[0] <== zeros[l];
        zeroHashers[l].inputs[1] <== zeros[l];
        zeros[l + 1] <== zeroHashers[l].out;
    }

    signal bits[MAX_DEPTH] <== Num2Bits(MAX_DEPTH)(size);

    component hashers[MAX_DEPTH];
    component muxes[MAX_DEPTH];
    signal nodes[MAX_DEPTH + 1];
    nodes[0] <== 0;

    for (var l = 0; l < MAX_DEPTH; l++) {
        // bit=0: current is left child, zeros[l] is right → hash(node, zeros[l])
        // bit=1: frontier[l] is left child, current is right → hash(frontier[l], node)
        muxes[l] = MultiMux1(2);
        muxes[l].c[0][0] <== nodes[l];
        muxes[l].c[0][1] <== frontier[l];
        muxes[l].c[1][0] <== zeros[l];
        muxes[l].c[1][1] <== nodes[l];
        muxes[l].s <== bits[l];

        hashers[l] = Poseidon(2);
        hashers[l].inputs[0] <== muxes[l].out[0];
        hashers[l].inputs[1] <== muxes[l].out[1];
        nodes[l + 1] <== hashers[l].out;
    }

    out <== nodes[MAX_DEPTH];
}

/// Batch insert N leaves into a standard fixed-depth IMT, verifying the root transition.
///
/// Computes zero hashes once and chains N sequential leaf insertions, threading the
/// frontier through each.  Only one RootFromFrontier call is needed (for the old root);
/// the new root is the output of the final insertion.
template MerkleTreeBatchInsert(MAX_DEPTH, N) {
    signal input root;
    signal input startIndex;
    signal input leaves[N];
    signal input initialFrontier[MAX_DEPTH];
    signal output out;

    // Verify old root
    component oldRootCalc = MerkleTreeRootFromFrontier(MAX_DEPTH);
    oldRootCalc.frontier <== initialFrontier;
    oldRootCalc.size <== startIndex;
    root === oldRootCalc.out;

    // Precompute zero hashes for use during insertions
    // TODO: Encode as constants
    component zeroHashers[MAX_DEPTH];
    signal zeros[MAX_DEPTH + 1];
    zeros[0] <== 0;
    for (var l = 0; l < MAX_DEPTH; l++) {
        zeroHashers[l] = Poseidon(2);
        zeroHashers[l].inputs[0] <== zeros[l];
        zeroHashers[l].inputs[1] <== zeros[l];
        zeros[l + 1] <== zeroHashers[l].out;
    }

    // Chain N insertions, threading frontier through each
    component num2bits[N];
    component hashers[N][MAX_DEPTH];
    component muxes[N][MAX_DEPTH];

    signal frontiers[N + 1][MAX_DEPTH];
    signal nodes[N][MAX_DEPTH + 1];
    signal frontierDeltas[N][MAX_DEPTH];

    for (var d = 0; d < MAX_DEPTH; d++) {
        frontiers[0][d] <== initialFrontier[d];
    }

    for (var i = 0; i < N; i++) {
        num2bits[i] = Num2Bits(MAX_DEPTH);
        num2bits[i].in <== startIndex + i;

        nodes[i][0] <== leaves[i];

        for (var l = 0; l < MAX_DEPTH; l++) {
            // bit=0: park current node, combine with zeros[l] for right sibling
            // bit=1: combine with stored left node (frontier[l])
            muxes[i][l] = MultiMux1(2);
            muxes[i][l].c[0][0] <== nodes[i][l];
            muxes[i][l].c[0][1] <== frontiers[i][l];
            muxes[i][l].c[1][0] <== zeros[l];
            muxes[i][l].c[1][1] <== nodes[i][l];
            muxes[i][l].s <== num2bits[i].out[l];

            hashers[i][l] = Poseidon(2);
            hashers[i][l].inputs[0] <== muxes[i][l].out[0];
            hashers[i][l].inputs[1] <== muxes[i][l].out[1];
            nodes[i][l + 1] <== hashers[i][l].out;

            // Frontier update: park node[l] when bit=0, keep frontier when bit=1
            // newFrontier[l] = nodes[l] + bit * (frontier[l] - nodes[l])
            frontierDeltas[i][l] <== num2bits[i].out[l] * (frontiers[i][l] - nodes[i][l]);
            frontiers[i + 1][l] <== nodes[i][l] + frontierDeltas[i][l];
        }
    }

    out <== nodes[N - 1][MAX_DEPTH];
}

/// Standard binary Merkle tree inclusion proof.
template MerkleTreeInclusion(MAX_DEPTH) {
    signal input leaf;
    signal input leafIndex;
    signal input siblings[MAX_DEPTH];
    signal output root;

    signal bits[MAX_DEPTH] <== Num2Bits(MAX_DEPTH)(leafIndex);
    signal nodes[MAX_DEPTH + 1];
    nodes[0] <== leaf;

    component hashers[MAX_DEPTH];
    component muxes[MAX_DEPTH];

    for (var i = 0; i < MAX_DEPTH; i++) {
        muxes[i] = MultiMux1(2);
        muxes[i].c[0][0] <== nodes[i];
        muxes[i].c[0][1] <== siblings[i];
        muxes[i].c[1][0] <== siblings[i];
        muxes[i].c[1][1] <== nodes[i];
        muxes[i].s <== bits[i];

        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== muxes[i].out[0];
        hashers[i].inputs[1] <== muxes[i].out[1];
        nodes[i + 1] <== hashers[i].out;
    }

    root <== nodes[MAX_DEPTH];
}

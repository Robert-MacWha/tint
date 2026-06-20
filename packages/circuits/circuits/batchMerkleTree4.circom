pragma circom 2.2.3;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/bitify.circom";
include "../node_modules/circomlib/circuits/mux2.circom";

/**
 * Builds a complete quaternary (4-ary) Merkle tree from 4^depth leaves and outputs the root.
 * Leaves at unused slots should be 0. All internal nodes are Poseidon(c0, c1, c2, c3).
 *
 * Flat storage: nodes[0..nLeaves-1] = leaves, then levels above packed consecutively.
 * Total nodes = nLeaves + (nLeaves-1)/3  (geometric series for arity 4).
 */
template BatchMerkleRoot4(depth) {
    var nLeaves = 1 << (2 * depth);        // 4^depth
    var nInternal = (nLeaves - 1) / 3;
    var nNodes = nLeaves + nInternal;

    signal input leaves[nLeaves];
    signal output root;

    signal nodes[nNodes];
    component poseidons[nInternal];

    for (var i = 0; i < nLeaves; i++) {
        nodes[i] <== leaves[i];
    }

    var levelStart = 0;
    var levelSize = nLeaves;
    var hashIdx = 0;

    while (levelSize > 1) {
        for (var j = 0; j < levelSize / 4; j++) {
            poseidons[hashIdx] = Poseidon(4);
            poseidons[hashIdx].inputs[0] <== nodes[levelStart + 4*j];
            poseidons[hashIdx].inputs[1] <== nodes[levelStart + 4*j + 1];
            poseidons[hashIdx].inputs[2] <== nodes[levelStart + 4*j + 2];
            poseidons[hashIdx].inputs[3] <== nodes[levelStart + 4*j + 3];
            nodes[levelStart + levelSize + j] <== poseidons[hashIdx].out;
            hashIdx++;
        }
        levelStart += levelSize;
        levelSize /= 4;
    }

    root <== nodes[nNodes - 1];
}

/**
 * Verifies that a leaf is included in a complete quaternary (4-ary) Merkle tree of depth `depth`.
 * The tree has 4^depth leaf slots. Each level contributes 2 bits of the leaf index and 3 siblings.
 *
 * siblings[i][0..2] are the 3 sibling children at level i, in positional order (skipping the
 * current node's position). leafIndex encodes the path as a base-4 number: bits [2i, 2i+1]
 * give the 2-bit position (0–3) of the current node among its 4 siblings at level i.
 *
 * MultiMux2(4) selects the ordered 4-child input to Poseidon using a 4×4 option matrix:
 * rows = child slot (0–3), columns = node position (selector value 0–3).
 * The diagonal holds `node`; off-diagonal holds the appropriate sibling.
 */
template BatchMerkleInclusion4(depth) {
    signal input leaf;
    signal input leafIndex;
    signal input siblings[depth][3];

    signal output root;

    signal nodes[depth + 1];
    nodes[0] <== leaf;

    component indexBits = Num2Bits(2 * depth);
    indexBits.in <== leafIndex;

    component mux[depth];
    component poseidons[depth];

    for (var i = 0; i < depth; i++) {
        mux[i] = MultiMux2(4);
        mux[i].s[0] <== indexBits.out[2 * i];
        mux[i].s[1] <== indexBits.out[2 * i + 1];

        // c[k][p]: value placed at child slot k when the current node sits at position p
        //   pos=0 → [node, s0, s1, s2]
        //   pos=1 → [s0, node, s1, s2]
        //   pos=2 → [s0, s1, node, s2]
        //   pos=3 → [s0, s1, s2, node]
        mux[i].c[0][0] <== nodes[i];         mux[i].c[0][1] <== siblings[i][0];
        mux[i].c[0][2] <== siblings[i][0];   mux[i].c[0][3] <== siblings[i][0];

        mux[i].c[1][0] <== siblings[i][0];   mux[i].c[1][1] <== nodes[i];
        mux[i].c[1][2] <== siblings[i][1];   mux[i].c[1][3] <== siblings[i][1];

        mux[i].c[2][0] <== siblings[i][1];   mux[i].c[2][1] <== siblings[i][1];
        mux[i].c[2][2] <== nodes[i];         mux[i].c[2][3] <== siblings[i][2];

        mux[i].c[3][0] <== siblings[i][2];   mux[i].c[3][1] <== siblings[i][2];
        mux[i].c[3][2] <== siblings[i][2];   mux[i].c[3][3] <== nodes[i];

        poseidons[i] = Poseidon(4);
        poseidons[i].inputs[0] <== mux[i].out[0];
        poseidons[i].inputs[1] <== mux[i].out[1];
        poseidons[i].inputs[2] <== mux[i].out[2];
        poseidons[i].inputs[3] <== mux[i].out[3];

        nodes[i + 1] <== poseidons[i].out;
    }

    root <== nodes[depth];
}

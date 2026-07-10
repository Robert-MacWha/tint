pragma circom 2.2.3;

include "./merkleTree.circom";
include "./comparators.circom";


/// Proves a batched insertion of new leaves into a Merkle tree.
///
/// The tree is divided into chunks of size 2^d, and the insertion is performed in two steps:
/// 1. The new leaves are merged into the current chunk at index `currentChunkIndex`, which may
///    be partially filled with `currentChunkFilled` existing leaves.
/// 2. Any overflow new leaves are inserted into the next chunk at index `currentChunkIndex + 1`.
template BatchChunkInsert(D, d) {
    var chunkSize = 1 << d;
    var pathLen = D - d;

    signal input oldRoot;                       // root of the tree before the insertion
    signal input newRoot;                       // root of the tree after the insertion
    signal input currentChunkFilled;            // how many leaves already in current chunk (0..chunkSize)
    signal input currentChunkIndex;             // 0-indexed position of current chunk in the D-d-level tree
    signal input existingLeaves[chunkSize];     // current chunk before batch (zero-padded)
    signal input newLeaves[chunkSize];          // new leaves to insert (zero-padded)
    signal input currentSiblings[pathLen];      // Merkle path for current chunk
    signal input nextSiblings[pathLen];         // Merkle path for next chunk (currentChunkIndex + 1)

    // --- Merge computation (current chunk) ---
    // computedCurrentChunk[pos] =
    //   existingLeaves[pos]                  if pos < currentChunkFilled
    //   newLeaves[pos - currentChunkFilled]  if pos >= currentChunkFilled (0 if no new leaf there)

    component isExistingPos[chunkSize];
    component isNewTargetCurrent[chunkSize][chunkSize]; // [pos][j]: (currentChunkFilled + j == pos)
    signal newContribCurrent[chunkSize][chunkSize + 1];
    signal existingPartCurrent[chunkSize];
    signal newPartCurrent[chunkSize];
    signal computedCurrentChunk[chunkSize];

    for (var pos = 0; pos < chunkSize; pos++) {
        isExistingPos[pos] = LessThan(7);
        isExistingPos[pos].in[0] <== pos;
        isExistingPos[pos].in[1] <== currentChunkFilled;

        newContribCurrent[pos][0] <== 0;
        for (var j = 0; j < chunkSize; j++) {
            isNewTargetCurrent[pos][j] = IsEqual();
            isNewTargetCurrent[pos][j].in[0] <== currentChunkFilled + j;
            isNewTargetCurrent[pos][j].in[1] <== pos;
            newContribCurrent[pos][j + 1] <== newContribCurrent[pos][j] +
                                              isNewTargetCurrent[pos][j].out * newLeaves[j];
        }

        existingPartCurrent[pos] <== isExistingPos[pos].out * existingLeaves[pos];
        newPartCurrent[pos] <== (1 - isExistingPos[pos].out) * newContribCurrent[pos][chunkSize];
        computedCurrentChunk[pos] <== existingPartCurrent[pos] + newPartCurrent[pos];
    }

    // --- Merge computation (next chunk / overflow) ---
    // computedNextChunk[pos] = newLeaves[chunkSize - currentChunkFilled + pos]
    //   if pos < currentChunkFilled (i.e., the leaf overflows), else 0.

    component isOverflow[chunkSize][chunkSize]; // [pos][j]: (currentChunkFilled + j == chunkSize + pos)
    signal newContribNext[chunkSize][chunkSize + 1];
    signal computedNextChunk[chunkSize];

    for (var pos = 0; pos < chunkSize; pos++) {
        newContribNext[pos][0] <== 0;
        for (var j = 0; j < chunkSize; j++) {
            isOverflow[pos][j] = IsEqual();
            isOverflow[pos][j].in[0] <== currentChunkFilled + j;
            isOverflow[pos][j].in[1] <== chunkSize + pos;
            newContribNext[pos][j + 1] <== newContribNext[pos][j] +
                                           isOverflow[pos][j].out * newLeaves[j];
        }
        computedNextChunk[pos] <== newContribNext[pos][chunkSize];
    }

    // --- Subtree hashing ---
    component oldCurrentRootCalc = BatchMerkleRoot(d);
    oldCurrentRootCalc.leaves <== existingLeaves;

    component newCurrentRootCalc = BatchMerkleRoot(d);
    newCurrentRootCalc.leaves <== computedCurrentChunk;

    component newNextRootCalc = BatchMerkleRoot(d);
    newNextRootCalc.leaves <== computedNextChunk;

    // Empty subtree root: zeros[d] = Poseidon^d(0)
    component zeroHashers[d];
    signal zeros[d + 1];
    zeros[0] <== 0;
    for (var l = 0; l < d; l++) {
        zeroHashers[l] = Poseidon(2);
        zeroHashers[l].inputs[0] <== zeros[l];
        zeroHashers[l].inputs[1] <== zeros[l];
        zeros[l + 1] <== zeroHashers[l].out;
    }

    // --- Two SubtreeUpdate proofs ---
    // Each SubtreeUpdate verifies old subtree root → new subtree root at a given index in the upper tree.
    // The same siblings are used for both old and new proofs (path is position-dependent, not value-dependent).

    signal intermediateRoot;

    // Proof 1: old current chunk root is in oldRoot
    component oldCurrentProof = MerkleTreeInclusion(pathLen);
    oldCurrentProof.leaf <== oldCurrentRootCalc.root;
    oldCurrentProof.leafIndex <== currentChunkIndex;
    oldCurrentProof.siblings <== currentSiblings;
    oldCurrentProof.root === oldRoot;

    // Proof 2: new current chunk root → intermediateRoot
    component newCurrentProof = MerkleTreeInclusion(pathLen);
    newCurrentProof.leaf <== newCurrentRootCalc.root;
    newCurrentProof.leafIndex <== currentChunkIndex;
    newCurrentProof.siblings <== currentSiblings;
    intermediateRoot <== newCurrentProof.root;

    // Proof 3: old next chunk (was empty = zeros[d]) is in intermediateRoot
    component oldNextProof = MerkleTreeInclusion(pathLen);
    oldNextProof.leaf <== zeros[d];
    oldNextProof.leafIndex <== currentChunkIndex + 1;
    oldNextProof.siblings <== nextSiblings;
    oldNextProof.root === intermediateRoot;

    // Proof 4: new next chunk root → newRoot
    component newNextProof = MerkleTreeInclusion(pathLen);
    newNextProof.leaf <== newNextRootCalc.root;
    newNextProof.leafIndex <== currentChunkIndex + 1;
    newNextProof.siblings <== nextSiblings;
    newNextProof.root === newRoot;
}

/// Inclusion proof for a leaf in a merkle tree of depth d.
template BatchMerkleInclusion(d) {
    signal input leaf;
    signal input leafIndex;
    signal input siblings[d];
    signal output root;

    signal bits[d] <== Num2Bits(d)(leafIndex);
    signal nodes[d + 1];
    nodes[0] <== leaf;

    component hashers[d];
    component muxes[d];

    for (var i = 0; i < d; i++) {
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

    root <== nodes[d];
}

/// Computes the root of a merkle tree of depth d from its leaves. 
///
/// Leaves must be zero-padded.
template BatchMerkleRoot(d) {
    var n = 1 << d;
    signal input leaves[n];
    signal output root;

    signal tree[2 * n - 1];
    component hashers[n - 1];

    for (var i = 0; i < n; i++) {
        tree[n - 1 + i] <== leaves[i];
    }

    for (var i = n - 2; i >= 0; i--) {
        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== tree[2 * i + 1];
        hashers[i].inputs[1] <== tree[2 * i + 2];
        tree[i] <== hashers[i].out;
    }

    root <== tree[0];
}
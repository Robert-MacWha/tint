pragma circom 2.2.0;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";
include "../node_modules/circomlib/circuits/mux1.circom";
include "../node_modules/circomlib/circuits/bitify.circom";

/// Computes a LeanIMT root from its frontier and size.
template LeanIMTRootFromFrontier(MAX_DEPTH) {
    signal input frontier[MAX_DEPTH];
    signal input size;
    signal output out;

    signal bits[MAX_DEPTH] <== Num2Bits(MAX_DEPTH)(size);

    signal started[MAX_DEPTH + 1];
    signal workingNode[MAX_DEPTH + 1];
    signal notStarted[MAX_DEPTH];
    signal notStartedButActive[MAX_DEPTH];
    signal startedAndActive[MAX_DEPTH];
    signal bootstrapDelta[MAX_DEPTH];
    signal mergeDelta[MAX_DEPTH];

    component hashers[MAX_DEPTH];

    started[0] <== 0;
    workingNode[0] <== 0;

    for (var i = 0; i < MAX_DEPTH; i++) {
        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== frontier[i];
        hashers[i].inputs[1] <== workingNode[i];

        notStarted[i]          <== 1 - started[i];
        notStartedButActive[i] <== notStarted[i] * bits[i];
        startedAndActive[i]    <== started[i] * bits[i];

        bootstrapDelta[i] <== notStartedButActive[i] * (frontier[i]    - workingNode[i]);
        mergeDelta[i]     <== startedAndActive[i]    * (hashers[i].out - workingNode[i]);

        workingNode[i + 1] <== workingNode[i] + bootstrapDelta[i] + mergeDelta[i];
        started[i + 1]     <== started[i] + notStartedButActive[i];
    }

    out <== workingNode[MAX_DEPTH];
}

/// Insert a leaf into a LeanIMT, updating the frontier.
template LeanIMTInsertLeaf(MAX_DEPTH) {
    signal input leafHash;
    signal input insertionIndex;
    signal input frontier[MAX_DEPTH];
    signal output newFrontier[MAX_DEPTH];

    signal bits[MAX_DEPTH] <== Num2Bits(MAX_DEPTH)(insertionIndex);
    signal node[MAX_DEPTH + 1];
    node[0] <== leafHash;

    signal parked[MAX_DEPTH + 1];
    signal notParkedAndMerging[MAX_DEPTH];
    signal notParkedAndParking[MAX_DEPTH];
    signal hashDelta[MAX_DEPTH];
    signal parkDelta[MAX_DEPTH];

    component hashers[MAX_DEPTH];

    parked[0] <== 0;

    for (var l = 0; l < MAX_DEPTH; l++) {
        hashers[l] = Poseidon(2);
        hashers[l].inputs[0] <== frontier[l];
        hashers[l].inputs[1] <== node[l];

        // notParkedAndMerging: we're still climbing AND bit is 1 → merge
        // notParkedAndParking: we're still climbing AND bit is 0 → park and stop
        notParkedAndMerging[l] <== (1 - parked[l]) * bits[l];
        notParkedAndParking[l] <== (1 - parked[l]) * (1 - bits[l]);

        // node climbs only if merging
        hashDelta[l] <== notParkedAndMerging[l] * (hashers[l].out - node[l]);
        node[l + 1] <== node[l] + hashDelta[l];

        // frontier[l] changes only if we're parking here
        parkDelta[l] <== notParkedAndParking[l] * (node[l] - frontier[l]);
        newFrontier[l] <== frontier[l] + parkDelta[l];

        // once parked, stay parked
        parked[l + 1] <== parked[l] + notParkedAndParking[l];
    }
}

/// Batch insert N leaves into a LeanIMT
template LeanIMTBatchInsert(MAX_DEPTH, N) {
    signal input root;       // root before batch
    signal input startIndex; // leaf count before batch
    signal input leaves[N];
    signal input initialFrontier[MAX_DEPTH];
    signal output out;

    // Verify oldRoot is consistent with initialFrontier + startIndex
    component oldRootCalc = LeanIMTRootFromFrontier(MAX_DEPTH);
    oldRootCalc.frontier <== initialFrontier;
    oldRootCalc.size <== startIndex;
    oldRootCalc.out === root;

    // Run N insertions, threading the frontier through
    component inserters[N];
    signal frontiers[N + 1][MAX_DEPTH];

    for (var d = 0; d < MAX_DEPTH; d++) {
        frontiers[0][d] <== initialFrontier[d];
    }

    for (var i = 0; i < N; i++) {
        inserters[i] = LeanIMTInsertLeaf(MAX_DEPTH);
        inserters[i].leafHash       <== leaves[i];
        inserters[i].insertionIndex <== startIndex + i;

        for (var d = 0; d < MAX_DEPTH; d++) {
            inserters[i].frontier[d] <== frontiers[i][d];
        }
        for (var d = 0; d < MAX_DEPTH; d++) {
            frontiers[i + 1][d] <== inserters[i].newFrontier[d];
        }
    }

    // Verify newRoot is consistent with final frontier + (startIndex + N)
    component newRootCalc = LeanIMTRootFromFrontier(MAX_DEPTH);
    newRootCalc.frontier <== frontiers[N];
    newRootCalc.size     <== startIndex + N;
    out <== newRootCalc.out;
}

template LeanIMTInclusion(MAX_DEPTH) {
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
        // bits[i] == 0 → current node is left  child: hash(node, sibling)
        // bits[i] == 1 → current node is right child: hash(sibling, node)
        muxes[i] = MultiMux1(2);
        muxes[i].c[0][0] <== nodes[i];     // left  input when bit=0
        muxes[i].c[0][1] <== siblings[i];  // left  input when bit=1
        muxes[i].c[1][0] <== siblings[i];  // right input when bit=0
        muxes[i].c[1][1] <== nodes[i];     // right input when bit=1
        muxes[i].s <== bits[i];

        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== muxes[i].out[0];
        hashers[i].inputs[1] <== muxes[i].out[1];

        nodes[i + 1] <== hashers[i].out;
    }

    root <== nodes[MAX_DEPTH];
}
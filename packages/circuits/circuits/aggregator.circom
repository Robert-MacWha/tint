pragma circom 2.2.3;

include "./batchMerkleTree4.circom";
include "./commitment.circom";
include "./amounts.circom";

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";

/// Aggregator circuit
///
/// The circuit checks the following:
/// 1. Verifies that the new provided root is correctly computed from the new leaves inserted into the old root.
/// 2. Each input note's nullifier is correctly computed from its nullifying key and
///    commitment.
/// 3. Each input note is included in either the new root.
/// 4. Each output note's commitment is correctly computed from its asset, amount, and partial commitment.
/// 5. The total amount of each asset in the input notes matches the total amount of that asset in the output notes.
/// 6. Each output note's asset matches at least one input note's asset.
/// 7. Verifies that leavesAggregationHash is the correct sequential Poseidon accumulation of all new leaves, excluding dummy leaves (0 for empty tree).
/// 8. Verifies that newRoot is the result of inserting the batch of new leaves into oldRoot at the specified batchIndex.
///
/// Dummy inputs: a slot with nullifiers[i] == 0 is treated as unused. The circuit
/// skips nullifier and merkle proof checks for that slot, and enforces assetsIn[i] == 0
/// and amountsIn[i] == 0 so dummy slots cannot affect balance or orphan checks.
///
/// The circuit assumes that the following checks are performed on-chain:
/// 1. That nullifiers have not already been used.
/// 2. That the same nullifier is not used more than once in a single transaction.
/// 3. That the same commitment is not used more than once in a single transaction.
/// 4. That the merkle trie roots are valid available roots.
/// 5. That a note is "spendable" (IE checking the spendability condition).
/// 6. Nullifiers recorded
/// 7. Unshields processed
/// 8. All amounts are within expected bounds (u128)
/// 9. That the merkle root will be updated on-chain.
/// 10. That leavesAggregationHash is a valid hash.
template Aggregator(nInputs, nOutputs, innerDepth, outerDepth) {
    var totalDepth = innerDepth + outerDepth;

    /// Public Signals
    signal input oldRoot;
    signal input newRoot;
    signal input leavesAggregationHash;             // Poseidon hash of all new leaves, excluding dummy leaves (0 for empty tree)
    signal input nullifiers[nInputs];               // Nullifiers for input notes (0 for dummy slot)
    signal input commitmentsOut[nOutputs];          // Commitments for output notes (0 for unshields)
    signal input unshieldAmounts[nOutputs];         // Amounts for unshield outputs (0 for internal transfers)
    signal input unshieldAssets[nOutputs];          // Assets for unshield outputs (0 for internal transfers)

    // TODO: Ensure this isn't optimized away - maybe also add as a private input and enforce equality?
    signal input boundParamsHash; // Hash of cryptographically bound data (IE spendability addresses, data, unshield recipients).  Used to ensure inputs can't be tampered with.

    /// Private Signals
    // Root update
    var nLeaves = 1 << (2 * innerDepth);
    signal input newLeaves[nLeaves];
    signal input startingAggregationHash;
    signal input batchIndex;
    signal input batchSiblings[outerDepth][3];

    // Input Notes
    signal input commitmentsIn[nInputs];
    signal input siblingsIn[nInputs][totalDepth][3];
    signal input leafIndicesIn[nInputs];
    signal input assetsIn[nInputs];
    signal input amountsIn[nInputs];
    signal input nullifyingKeysIn[nInputs];
    signal input partialCommitmentsIn[nInputs];

    // Output notes
    signal input assetsOut[nOutputs];
    signal input amountsOut[nOutputs];
    signal input partialCommitmentsOut[nOutputs];

    // ~10k non-linear constraints + ~20k linear constraints
    component checkLeavesAggregation = CheckLeavesAggregationHash(nLeaves);
    checkLeavesAggregation.startingHash <== startingAggregationHash;
    checkLeavesAggregation.endingHash <== leavesAggregationHash;
    checkLeavesAggregation.leaves <== newLeaves;

    // ~30k non-linear constraints + ~25k linear constraints
    component checkRootUpdate = CheckRootUpdate(innerDepth, outerDepth);
    checkRootUpdate.oldRoot <== oldRoot;
    checkRootUpdate.newRoot <== newRoot;
    checkRootUpdate.newLeaves <== newLeaves;
    checkRootUpdate.batchIndex <== batchIndex;
    checkRootUpdate.batchSiblings <== batchSiblings;

    // Negligable
    component checkDummyInputs = CheckDummyInputs(nInputs);
    checkDummyInputs.nullifiers <== nullifiers;
    checkDummyInputs.assetsIn <== assetsIn;
    checkDummyInputs.amountsIn <== amountsIn;

    // ~10k non-linear constraints + ~2k linear constraints
    component checkNullifiers = CheckNullifiers(nInputs);
    checkNullifiers.nullifiers <== nullifiers;
    checkNullifiers.nullifyingKeysIn <== nullifyingKeysIn;
    checkNullifiers.leafIndicesIn <== leafIndicesIn;

    // ~10k non-linear constraints + ~2k linear constraints
    component checkCommitments = CheckCommitments(nOutputs);
    checkCommitments.commitmentsOut <== commitmentsOut;
    checkCommitments.assetsOut <== assetsOut;
    checkCommitments.amountsOut <== amountsOut;
    checkCommitments.partialCommitmentsOut <== partialCommitmentsOut;

    // ~60k non-linear constraints + ~65k linear constraints
    component checkMerkleProofs = CheckMerkleProofs(nInputs, totalDepth);
    checkMerkleProofs.root <== newRoot;
    checkMerkleProofs.nullifiers <== nullifiers;
    checkMerkleProofs.siblingsIn <== siblingsIn;
    checkMerkleProofs.leafIndicesIn <== leafIndicesIn;
    checkMerkleProofs.assetsIn <== assetsIn;
    checkMerkleProofs.amountsIn <== amountsIn;
    checkMerkleProofs.partialCommitmentsIn <== partialCommitmentsIn;

    // Negligable
    component checkUnshields = CheckUnshields(nOutputs);
    checkUnshields.commitmentsOut <== commitmentsOut;
    checkUnshields.unshieldAmounts <== unshieldAmounts;
    checkUnshields.unshieldAssets <== unshieldAssets;
    checkUnshields.assetsOut <== assetsOut;
    checkUnshields.amountsOut <== amountsOut;

    // Negligable
    component checkBalance = CheckInputsBalanceOutputs(nInputs, nOutputs);
    checkBalance.assetsIn <== assetsIn;
    checkBalance.amountsIn <== amountsIn;
    checkBalance.assetsOut <== assetsOut;
    checkBalance.amountsOut <== amountsOut;

    // Negligable
    component checkNoOrphans = CheckNoOrphanOutputs(nInputs, nOutputs);
    checkNoOrphans.assetsIn <== assetsIn;
    checkNoOrphans.assetsOut <== assetsOut;
}

/// Enforce that leavesAggregationHash is the correct sequential Poseidon accumulation
/// of all non-zero leaves starting from startingHash. Zero leaves are skipped.
template CheckLeavesAggregationHash(nLeaves) {
    signal input startingHash;
    signal input endingHash;
    signal input leaves[nLeaves];

    component isDummy[nLeaves];
    component poseidons[nLeaves];
    signal runningHash[nLeaves + 1];
    signal delta[nLeaves];

    runningHash[0] <== startingHash;

    for (var i = 0; i < nLeaves; i++) {
        isDummy[i] = IsZero();
        isDummy[i].in <== leaves[i];

        poseidons[i] = Poseidon(2);
        poseidons[i].inputs[0] <== runningHash[i];
        poseidons[i].inputs[1] <== leaves[i];

        // If leaf is zero, keep the hash unchanged; otherwise advance it.
        delta[i] <== (1 - isDummy[i].out) * (poseidons[i].out - runningHash[i]);
        runningHash[i + 1] <== runningHash[i] + delta[i];
    }

    endingHash === runningHash[nLeaves];
}

/// Enforce that the new root is the old root with the new leaves inserted.
///
/// The inner batch tree (innerDepth) hashes newLeaves into a single batchRoot.
/// The outer accumulator (outerDepth) proves batchRoot was inserted at batchIndex,
/// replacing 0, by showing the same sibling path reconstructs oldRoot (with 0) and
/// newRoot (with batchRoot). Shared siblings guarantee that is the only change.
template CheckRootUpdate(innerDepth, outerDepth) {
    var nLeaves = 1 << (2 * innerDepth);

    signal input oldRoot;
    signal input newRoot;
    signal input newLeaves[nLeaves];
    signal input batchIndex;
    signal input batchSiblings[outerDepth][3];

    component batchTree = BatchMerkleRoot4(innerDepth);
    batchTree.leaves <== newLeaves;

    // Old slot was 0 (append-only outer tree)
    component oldInclusion = BatchMerkleInclusion4(outerDepth);
    oldInclusion.leaf <== 0;
    oldInclusion.leafIndex <== batchIndex;
    oldInclusion.siblings <== batchSiblings;
    oldInclusion.root === oldRoot;

    // New slot holds the batch root
    component newInclusion = BatchMerkleInclusion4(outerDepth);
    newInclusion.leaf <== batchTree.root;
    newInclusion.leafIndex <== batchIndex;
    newInclusion.siblings <== batchSiblings;
    newInclusion.root === newRoot;
}

/// Enforce that dummy input slots (nullifiers[i] == 0) have zero asset and zero amount.
/// This prevents dummy slots from affecting balance or orphan output checks.
template CheckDummyInputs(nInputs) {
    signal input nullifiers[nInputs];
    signal input assetsIn[nInputs];
    signal input amountsIn[nInputs];

    component isDummy[nInputs];

    for (var i = 0; i < nInputs; i++) {
        isDummy[i] = IsZero();
        isDummy[i].in <== nullifiers[i];
        isDummy[i].out * assetsIn[i] === 0;
        isDummy[i].out * amountsIn[i] === 0;
    }
}

/// Enforce that for non-dummy input slots, nullifiers are correctly computed as 
/// Poseidon(nullifyingKey, leafIndex).
template CheckNullifiers(nInputs) {
    signal input nullifiers[nInputs];
    signal input nullifyingKeysIn[nInputs];
    signal input leafIndicesIn[nInputs];

    component hasher[nInputs];
    component isDummy[nInputs];

    for (var i = 0; i < nInputs; i++) {
        isDummy[i] = IsZero();
        isDummy[i].in <== nullifiers[i];

        hasher[i] = Poseidon(2);
        hasher[i].inputs[0] <== nullifyingKeysIn[i];
        hasher[i].inputs[1] <== leafIndicesIn[i];

        // Skip nullifier check for dummy slots.
        (1 - isDummy[i].out) * (hasher[i].out - nullifiers[i]) === 0;
    }
}

/// Enforce that commitments are correctly computed as Poseidon(asset, amount, partialCommitment).
template CheckCommitments(nOutputs) {
    signal input commitmentsOut[nOutputs];
    signal input assetsOut[nOutputs];
    signal input amountsOut[nOutputs];
    signal input partialCommitmentsOut[nOutputs];

    component isDummy[nOutputs];
    component commitmentHasher[nOutputs];

    for (var i = 0; i < nOutputs; i++) {
        isDummy[i] = IsZero();
        isDummy[i].in <== commitmentsOut[i];

        commitmentHasher[i] = CommitmentHasher();
        commitmentHasher[i].asset <== assetsOut[i];
        commitmentHasher[i].amount <== amountsOut[i];
        commitmentHasher[i].partialCommitment <== partialCommitmentsOut[i];

        // If not a dummy slot, enforce that the computed commitment matches the provided commitment.
        (1 - isDummy[i].out) * (commitmentHasher[i].commitment - commitmentsOut[i]) === 0;
    }
}

/// Enforce that for non-dummy input slots, the input note commtiment is included in the
/// new root.
template CheckMerkleProofs(nInputs, depth) {
    signal input root;
    signal input nullifiers[nInputs];

    signal input leafIndicesIn[nInputs];
    signal input siblingsIn[nInputs][depth][3];
    signal input assetsIn[nInputs];
    signal input amountsIn[nInputs];
    signal input partialCommitmentsIn[nInputs];

    component isDummy[nInputs];
    component commitmentHashers[nInputs];
    component merkleVerifiers[nInputs];
    signal rootComputed[nInputs];

    for (var i = 0; i < nInputs; i++) {
        isDummy[i] = IsZero();
        isDummy[i].in <== nullifiers[i];

        commitmentHashers[i] = CommitmentHasher();
        commitmentHashers[i].asset <== assetsIn[i];
        commitmentHashers[i].amount <== amountsIn[i];
        commitmentHashers[i].partialCommitment <== partialCommitmentsIn[i];

        merkleVerifiers[i] = BatchMerkleInclusion4(depth);
        merkleVerifiers[i].leaf <== commitmentHashers[i].commitment;
        merkleVerifiers[i].leafIndex <== leafIndicesIn[i];
        merkleVerifiers[i].siblings <== siblingsIn[i];

        // If not a dummy slot, enforce that the computed root matches the provided root.
        (1 - isDummy[i].out) * (merkleVerifiers[i].root - root) === 0;
    }
}

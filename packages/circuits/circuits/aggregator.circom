pragma circom 2.2.3;

// include "./commitment.circom";
include "./merkleTree.circom";

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";

/// Aggregator circuit
///
/// Verifies that a set of input notes can all be spent, and that the resulting output notes are
/// correctly formed and balance with the inputs.
///
/// The circuit checks the following:
/// 1. Each input note's nullifier is correctly computed from its nullifying key and
///    commitment.
/// 2. Each input note is included in either the staging or archive merkle tree.
/// 3. Each output note's commitment is correctly computed from its asset, amount, and partial commitment.
/// 4. The total amount of each asset in the input notes matches the total amount of that asset in the output notes.
/// 5. Each output note's asset matches at least one input note's asset.
/// 
/// The circuit assumes that the following checks are performed on-chain:
/// 1. That nullifiers have not already been used.
/// 2. That the same nullifier is not used more than once in a single transaction.
/// 3. That the same commitment is not used more than once in a single transaction.
/// 4. That the merkle trie roots are valid available roots.
/// 5. That a note is "spendable" (IE checking the spendability condition).
/// 7. Nullifiers recorded
/// 8. Unshields processed
template Aggregator(nInputs, nOutputs, stagingTreeDepth, archiveTreeDepth) {

    /// Public Signals
    signal input stagingRoot;                       // Staging merkle tree root
    signal input archiveRoot;                       // Archive merkle tree root
    signal input nullifiers[nInputs];               // Nullifiers for input notes
    signal input commitmentsOut[nOutputs];          // Commitments for output notes (0 for unshields)
    signal input unshieldRecipients[nOutputs];      // Recipients for unshield outputs (0 for internal transfers)
    signal input unshieldAmounts[nOutputs];         // Amounts for unshield outputs (0 for internal transfers)
    signal input unshieldAssets[nOutputs];          // Assets for unshield outputs (0 for internal transfers)
    signal input spendabilityAddressesIn[nInputs];  // Spendability addresses for input notes
    signal input spendabilityDataIn[nInputs];       // Spendability data for input notes

    /// Private Signals
    // Input Notes
    signal input stagingPathElementsIn[nInputs][stagingTreeDepth];
    signal input archivePathElementsIn[nInputs][archiveTreeDepth];
    signal input stagingLeafIndicesIn[nInputs];
    signal input archiveLeafIndicesIn[nInputs];
    signal input assetsIn[nInputs];
    signal input amountsIn[nInputs];
    signal input randomsIn[nInputs];
    signal input nullifyingKeysIn[nInputs];

    // Output notes
    signal input assetsOut[nOutputs];
    signal input amountsOut[nOutputs];

    //? poseidon(randomOut, nullifyingPubKey, spendabilityAddress, spendabilityData)
    signal input partialCommitmentsOut[nOutputs];

    component computeCommitments = ComputeInputCommitments(nInputs);
    computeCommitments.assetsIn <== assetsIn;
    computeCommitments.amountsIn <== amountsIn;
    computeCommitments.randomsIn <== randomsIn;
    computeCommitments.nullifyingKeysIn <== nullifyingKeysIn;
    computeCommitments.spendabilityAddressesIn <== spendabilityAddressesIn;
    computeCommitments.spendabilityDataIn <== spendabilityDataIn;

    component checkNullifiers = CheckNullifiers(nInputs);
    checkNullifiers.nullifiers <== nullifiers;
    checkNullifiers.nullifyingKeysIn <== nullifyingKeysIn;
    checkNullifiers.commitmentsIn <== computeCommitments.commitments;

    component checkMerkleProofs = CheckMerkleProofs(nInputs, stagingTreeDepth, archiveTreeDepth);
    checkMerkleProofs.stagingRoot <== stagingRoot;
    checkMerkleProofs.archiveRoot <== archiveRoot;
    checkMerkleProofs.stagingPathElementsIn <== stagingPathElementsIn;
    checkMerkleProofs.archivePathElementsIn <== archivePathElementsIn;
    checkMerkleProofs.stagingLeafIndicesIn <== stagingLeafIndicesIn;
    checkMerkleProofs.archiveLeafIndicesIn <== archiveLeafIndicesIn;
    checkMerkleProofs.commitmentsIn <== computeCommitments.commitments;

    component checkCommitments = CheckCommitments(nOutputs);
    checkCommitments.commitmentsOut <== commitmentsOut;
    checkCommitments.assetsOut <== assetsOut;
    checkCommitments.amountsOut <== amountsOut;
    checkCommitments.partialCommitmentsOut <== partialCommitmentsOut;

    component checkUnshields = CheckUnshields(nOutputs);
    checkUnshields.commitmentsOut <== commitmentsOut;
    checkUnshields.unshieldRecipients <== unshieldRecipients;
    checkUnshields.unshieldAmounts <== unshieldAmounts;
    checkUnshields.unshieldAssets <== unshieldAssets;
    checkUnshields.assetsOut <== assetsOut;
    checkUnshields.amountsOut <== amountsOut;

    component checkBalance = CheckInputsBalanceOutputs(nInputs, nOutputs);
    checkBalance.assetsIn <== assetsIn;
    checkBalance.amountsIn <== amountsIn;
    checkBalance.assetsOut <== assetsOut;
    checkBalance.amountsOut <== amountsOut;

    component checkNoOrphans = CheckNoOrphanOutputs(nInputs, nOutputs);
    checkNoOrphans.assetsIn <== assetsIn;
    checkNoOrphans.assetsOut <== assetsOut;

    component checkAmountLimits = CheckAmountsWithinLimits(nInputs, nOutputs);
    checkAmountLimits.amountsIn <== amountsIn;
    checkAmountLimits.amountsOut <== amountsOut;
}

template ComputeInputCommitments(nInputs) {
    signal input assetsIn[nInputs];
    signal input amountsIn[nInputs];
    signal input randomsIn[nInputs];
    signal input nullifyingKeysIn[nInputs];
    signal input spendabilityAddressesIn[nInputs];
    signal input spendabilityDataIn[nInputs];

    signal output commitments[nInputs];

    component pubKeyDeriver[nInputs];
    component partialCommitmentHasher[nInputs];
    component commitmentHasher[nInputs];

    for (var i = 0; i < nInputs; i++) {
        pubKeyDeriver[i] = Poseidon(1);
        pubKeyDeriver[i].inputs[0] <== nullifyingKeysIn[i];

        partialCommitmentHasher[i] = PartialCommitmentHasher();
        partialCommitmentHasher[i].random <== randomsIn[i];
        partialCommitmentHasher[i].nullifyingPubKey <== pubKeyDeriver[i].out;
        partialCommitmentHasher[i].spendabilityAddress <== spendabilityAddressesIn[i];
        partialCommitmentHasher[i].spendabilityData <== spendabilityDataIn[i];

        commitmentHasher[i] = CommitmentHasher();
        commitmentHasher[i].asset <== assetsIn[i];
        commitmentHasher[i].amount <== amountsIn[i];
        commitmentHasher[i].partialCommitment <== partialCommitmentHasher[i].partialCommitment;

        commitments[i] <== commitmentHasher[i].commitment;
    }
}

template CheckNullifiers(nInputs) {
    signal input nullifiers[nInputs];
    signal input nullifyingKeysIn[nInputs];
    signal input commitmentsIn[nInputs];

    component hasher[nInputs];
    for (var i = 0; i < nInputs; i++) {
        hasher[i] = Poseidon(2);
        hasher[i].inputs[0] <== nullifyingKeysIn[i];
        hasher[i].inputs[1] <== commitmentsIn[i];
        hasher[i].out === nullifiers[i];
    }
}

template CheckMerkleProofs(nInputs, stagingTreeDepth, archiveTreeDepth) {
    signal input stagingRoot;
    signal input archiveRoot;

    signal input stagingPathElementsIn[nInputs][stagingTreeDepth];
    signal input archivePathElementsIn[nInputs][archiveTreeDepth];
    signal input stagingLeafIndicesIn[nInputs];
    signal input archiveLeafIndicesIn[nInputs];
    signal input commitmentsIn[nInputs];

    component stagingMerkleVerifier[nInputs];
    component archiveMerkleVerifier[nInputs];
    signal stagingRootComputed[nInputs];
    signal archiveRootComputed[nInputs];

    for (var i = 0; i < nInputs; i++) {
        stagingMerkleVerifier[i] = LeanIMTProofVerifier(stagingTreeDepth);
        stagingMerkleVerifier[i].leaf <== commitmentsIn[i];
        stagingMerkleVerifier[i].leafIndex <== stagingLeafIndicesIn[i];
        stagingMerkleVerifier[i].siblings <== stagingPathElementsIn[i];

        archiveMerkleVerifier[i] = LeanIMTProofVerifier(archiveTreeDepth);
        archiveMerkleVerifier[i].leaf <== commitmentsIn[i];
        archiveMerkleVerifier[i].leafIndex <== archiveLeafIndicesIn[i];
        archiveMerkleVerifier[i].siblings <== archivePathElementsIn[i];

        stagingRootComputed[i] <== stagingMerkleVerifier[i].root;
        archiveRootComputed[i] <== archiveMerkleVerifier[i].root;

        (stagingRootComputed[i] - stagingRoot) * (archiveRootComputed[i] - archiveRoot) === 0;
    }
}

template CheckCommitments(nOutputs) {
    signal input commitmentsOut[nOutputs];
    signal input assetsOut[nOutputs];
    signal input amountsOut[nOutputs];
    signal input partialCommitmentsOut[nOutputs];

    component isUnshield[nOutputs];
    component commitmentHasher[nOutputs];

    for (var i = 0; i < nOutputs; i++) {
        isUnshield[i] = IsZero();
        isUnshield[i].in <== commitmentsOut[i];

        commitmentHasher[i] = CommitmentHasher();
        commitmentHasher[i].asset <== assetsOut[i];
        commitmentHasher[i].amount <== amountsOut[i];
        commitmentHasher[i].partialCommitment <== partialCommitmentsOut[i];

        // Internal transfer: commitment must equal the computed hash
        (1 - isUnshield[i].out) * (commitmentHasher[i].commitment - commitmentsOut[i]) === 0;
    }
}

template CheckUnshields(nOutputs) {
    signal input commitmentsOut[nOutputs];
    signal input unshieldRecipients[nOutputs];
    signal input unshieldAmounts[nOutputs];
    signal input unshieldAssets[nOutputs];
    signal input assetsOut[nOutputs];
    signal input amountsOut[nOutputs];

    component isUnshield[nOutputs];

    for (var i = 0; i < nOutputs; i++) {
        isUnshield[i] = IsZero();
        isUnshield[i].in <== commitmentsOut[i];

        // Internal transfer: unshield fields must be zero
        (1 - isUnshield[i].out) * unshieldRecipients[i] === 0;
        (1 - isUnshield[i].out) * unshieldAmounts[i] === 0;
        (1 - isUnshield[i].out) * unshieldAssets[i] === 0;
        // Unshield: public amounts and assets must bind to the private values used in balance check
        isUnshield[i].out * (unshieldAmounts[i] - amountsOut[i]) === 0;
        isUnshield[i].out * (unshieldAssets[i] - assetsOut[i]) === 0;
    }
}

/// Check that for each input asset, the total input amount matches the total output amount for that asset
template CheckInputsBalanceOutputs(nInputs, nOutputs) {
    signal input assetsIn[nInputs];
    signal input amountsIn[nInputs];

    signal input assetsOut[nOutputs];
    signal input amountsOut[nOutputs];

    component assetMatchIn[nInputs][nInputs];
    component assetMatchOut[nInputs][nOutputs];
    signal weightedIn[nInputs][nInputs];
    signal weightedOut[nInputs][nOutputs];

    for (var i = 0; i < nInputs; i++) {
        var input_sum = 0;
        var output_sum = 0;

        for (var j = 0; j < nInputs; j++) {
            assetMatchIn[i][j] = IsEqual();
            assetMatchIn[i][j].in[0] <== assetsIn[i];
            assetMatchIn[i][j].in[1] <== assetsIn[j];
            weightedIn[i][j] <== assetMatchIn[i][j].out * amountsIn[j];
            input_sum += weightedIn[i][j];
        }

        for (var k = 0; k < nOutputs; k++) {
            assetMatchOut[i][k] = IsEqual();
            assetMatchOut[i][k].in[0] <== assetsIn[i];
            assetMatchOut[i][k].in[1] <== assetsOut[k];
            weightedOut[i][k] <== assetMatchOut[i][k].out * amountsOut[k];
            output_sum += weightedOut[i][k];
        }

        input_sum === output_sum;
    }
}

/// Check that for each output asset, there is at least one matching input asset
template CheckNoOrphanOutputs(nInputs, nOutputs) {
    signal input assetsIn[nInputs];
    signal input assetsOut[nOutputs];

    component assetMatch[nOutputs][nInputs];
    component hasMatch[nOutputs];

    for (var i = 0; i < nOutputs; i++) {
        var matches = 0;

        for (var j = 0; j < nInputs; j++) {
            assetMatch[i][j] = IsEqual();
            assetMatch[i][j].in[0] <== assetsOut[i];
            assetMatch[i][j].in[1] <== assetsIn[j];
            matches += assetMatch[i][j].out;
        }

        hasMatch[i] = IsZero();
        hasMatch[i].in <== matches;
        hasMatch[i].out === 0;
    }
}

template CheckAmountsWithinLimits(nInputs, nOutputs) {
    signal input amountsIn[nInputs];
    signal input amountsOut[nOutputs];

    component checkIn[nInputs];
    component checkOut[nOutputs];

    for (var i = 0; i < nInputs; i++) {
        checkIn[i] = Check128();
        checkIn[i].x <== amountsIn[i];
    }

    for (var j = 0; j < nOutputs; j++) {
        checkOut[j] = Check128();
        checkOut[j].x <== amountsOut[j];
    }
}

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

template Check128() {
    signal input x;
    
    // Instantiating Num2Bits with 128 enforces that the input can be 
    // decomposed into exactly 128 binary bits.
    component n2b = Num2Bits(128);
    n2b.in <== x;
}
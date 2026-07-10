pragma circom 2.2.3;

include "./comparators.circom";

/// For unshields (commitmentsOut[i] == 0), enforce that the public amount and 
/// asset match the private unshield values. For internal transfers 
/// (commitmentsOut[i] != 0), enforce that unshield amounts and assets are zero.
template CheckUnshields(nOutputs) {
    signal input commitmentsOut[nOutputs];
    signal input unshieldAmounts[nOutputs];
    signal input unshieldAssets[nOutputs];
    signal input assetsOut[nOutputs];
    signal input amountsOut[nOutputs];

    component isUnshield[nOutputs];

    for (var i = 0; i < nOutputs; i++) {
        isUnshield[i] = IsZero();
        isUnshield[i].in <== commitmentsOut[i];

        // Internal transfer: unshield fields must be zero
        (1 - isUnshield[i].out) * unshieldAmounts[i] === 0;
        (1 - isUnshield[i].out) * unshieldAssets[i] === 0;
        // Unshield: public amounts and assets must bind to the private values used in balance check
        isUnshield[i].out * (unshieldAmounts[i] - amountsOut[i]) === 0;
        isUnshield[i].out * (unshieldAssets[i] - assetsOut[i]) === 0;
    }
}

/// Check that for each input asset, the total input amount matches the total 
/// output amount for that asset.
/// 
/// Does NOT enforce that output assets are a subset of input assets; use 
/// CheckNoOrphanOutputs for that.
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

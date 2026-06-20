template DummySpendability(nOutputs) {
    /// Public Signals
    signal input nullifier;
    signal input commitmentsOut[nOutputs];

    /// Private Signals
    signal input assetsOut[nOutputs];
    signal input amountsOut[nOutputs];
    signal input randomsOut[nOutputs];
    signal input nullifyingPubKeysOut[nOutputs];
    signal input spendabilityAddresseOut[nOutputs];
    signal input spendabilityDataOut[nOutputs];
}
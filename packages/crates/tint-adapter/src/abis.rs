use alloy_sol_types::sol;

sol! {
    /// A call the adapter makes after executing the operation.
    #[derive(Debug, PartialEq, Eq)]
    struct Call {
        address target;
        uint256 value;
        bytes data;
    }

    /// Shields the adapter's whole balance of `asset` as a note for `partialCommitment`.
    #[derive(Debug, PartialEq, Eq)]
    struct Shield {
        address asset;
        bytes32 partialCommitment;
        bytes encryptedPartial;
    }
}

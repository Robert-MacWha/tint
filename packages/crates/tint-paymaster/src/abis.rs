use alloy_sol_types::sol;

sol! {
    enum RefundKind { None, Transfer, Deposit }

    struct Refund {
        RefundKind kind;
        address recipient;
        bytes32 partialCommitment;
        bytes encryptedPartial;
    }
}

use alloy::sol;

sol! {
    contract Tint {
        event Deposited(
            address indexed asset,
            uint128 amount,
            address indexed spendabilityAddress,
            bytes spendabilityData,
            bytes32 commitment
        );
        event Committed(
            bytes32 indexed commitment,
            address indexed spendabilityAddress,
            bytes spendabilityData
        );
        event Nullified(bytes32 indexed nullifier);
        event Withdrawn(
            address indexed asset,
            uint128 amount,
            address indexed recipient
        );
    }
}

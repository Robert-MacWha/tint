## Blockers

1. State access.  We need to guarantee that the list of staged operations is the same list that's proven against in-circuit.  This means that a summary of this list of staged operations needs to be stored verifiably on-chain.  This could be done via progressively hashing the list on-chain into an aggregation hash, or by storing the operations in blob storage.  
   1. Using an aggregation hash introduces much higher gas costs (each operation needs to upload multiple proofs.  Since we can't use groth16 those proofs will be several kilobytes).
   2. Using blob storage is cheaper on-chain, but requires proving the blob hash in-circuit, which is probably incredibly expensive because it uses sha256 and different field arithmetics.

## Definitions

**Operation**: An operation is a single interaction with the protocol.  Operations can be shields, transfers, or unshield.

**Staged Operation**: A staged operation is an operation that has been submitted to the protocol, but not verified in-circuit yet.  Staged operations are emitted as events and cryptographically bound via the aggregation hash.

**Aggregation Hash**: The aggregation hash is a progressive hash of all operations.  Aggregation Hash [N] is computed as `hash(Aggregation Hash [N-1], Operation Hash [N])`.

**ZK State**: The zk state is the private state of the protocol.  It is made up of the merkle tree of UTXO notes.

**Commitment**: Commitment is the process of applying a list of staged operations to the protocol state.  Commitment is performed in-circuit, and the resulting new merkle root is stored on-chain.  Commitment is required for the smart contract to verify the **zk state** of the protocol.

## Why do proofs get evaluated on-chain?

1. So that users can verify that the current state of the protocol is valid without themselves verifying the entire history of the protocol.
2. So that the protocol can verify its **zk state** (IE that a user has sufficient funds for a given operation).

Point 1 is convenience, but not strictly required.  Point 2 is only required when the protocol needs to verify **zk state**, which only really happens during unshields.

So what happens if you only evaluate proofs on-chain during unshields?  The protocol can still record **operations**, but will defer **commitment** of these **operations** until the next unshield.  This means that:
1. Unshield proofs are orders of magnitude larger.  Instead of just proving a single operation, the unshield proof must verify an arbitrary number of operations that have occured since the last **commitment**.
2. Shields and transfers are essentially free.  Both of these **operations** can be verified off-chain, so the protocol can simply record the events.  If an invalid event is submitted, users will simply ignore that event since it will not be a valid proof.

## Technical Outline

```solidity
contract Tint {
    /// Shields an asset into the protocol.  Stages the shield for later verification.
    function shield(address asset, uint256 amount, bytes32 partialCommitment) external {
        erc20(asset).transferFrom(msg.sender, address(this), amount);

        bytes32 commitmentHash = toCommitmentHash(asset, amount, partialCommitment);
        _stageOperation(commitmentHash);
        emit Shield(
            asset,
            amount,
            partialCommitment
        );
    }

    /// Perform a shielded operation within the protocol.  Stages the operation for later verification.
    function operate(Operation calldata operation) external {
        bytes32 operationHash = toOperationHash(operation);
        _stageOperation(operationHash);
        emit Operated(
            operation
        );
    }

    /// Unshields an asset from the protocol.  Verifies all staged operations since the last unshield.
    function unshield(Unshield calldata unshield) external {
        /// 1. Get the last committed aggregation hash from storage.
        /// 2. Get the "target" aggregation hash from storage.  All staged operations up to this point will be verified in-circuit.
        /// 3. Verify the unshield proof in-circuit.  This proof will:
        ///    a. Recursively verify that the set of operations being committed match the operations between the last committed aggregation hash and the target aggregation hash.
        ///    b. Apply all of the operations to the protocol state, performing many merkle tree appends.
        ///    c. Verify that the unshield proof is valid given the new protocol state

        emit Unshielded(
            unshield.asset,
            unshield.amount,
            unshield.recipient
        );
        erc20(unshield.asset).transfer(unshield.recipient, unshield.amount);
    }

    function _stageOperation(bytes32 operationHash) internal {
        // Use an aggregation hash to "stage" the operation hash for later verification in-circuit.
    }
}
```

```circuit
Unshield Circuit

public inputs:
    /// A previously committed aggregation hash. From smart contract storage
    - committed_aggregation_hash: Fr
    /// The target aggregation hash we are committing to. From smart contract storage
    - target_aggregation_hash: Fr
    /// A previously committed merkle root. From smart contract storage
    - committed_merkle_root: Fr
    /// A previously committed nullifier root. From smart contract storage
    - committed_nullifier_root: Fr
    - unshield_asset: Fr
    - unshield_amount: Fr
    - unshield_recipient: Fr
    /// Can bind more data to the operation if we want

private inputs:
    - staged_operations: [Operation; N]

constraints:
    - Verify that the staged_operations hashes match the aggregation hashes between committed_aggregation_hash and target_aggregation_hash
        ```
        aggregation_hash = committed_aggregation_hash
        for operation in staged_operations:
            aggregation_hash = hash(aggregation_hash, operation.hash())
        assert aggregation_hash == target_aggregation_hash
        ```
    - Recursively verify each operation, accumulating the merkle tree updates as we go.
        - Verify that the operation's input commitments are present in the tree
        - Verify that the operation's spent nullifiers are computed correctly
        - Verify that the operation's output commitments are computed correctly
        - Verify that the operation has valid balance changes
        - Recursively verify spendability for each input commitment
            - Whatever
        - Update the merkle tree & nullifier sparse merkle tree with the operation's outputs and nullifiers

Outputs
    - new_merkle_root: Fr
    - new_nullifier_root: Fr
```

## Challenges

1. The unshield proof will be *very* expensive to compute. Using a recursive proof system will be a must (something like plonky2 or flock).
2. Unshields will be gas-heavy.  There is NO WAY we can use groth16 for this at any resonable timescale.  Using plonky2 proof verification gas costs are ~1m gas, which isn't actually terrible (railgun currently averages ~1.2m gas for a shielded transfer).
3. Nullifiers need to be stored in **zk state**, probably as a sparse merkle tree.  This is because we need to verify that nullifiers are valid before recording them.

## Ideas

1. We need to **commit** **operations** to perform unshields, but we don't need to perform unshields for a user to withdraw funds from the protocol.  Using p2p swaps, we could allow user (a) to swap their private funds with user (b)'s public funds.  This would amortize the cost of unshielding across multiple users, and allow users to withdraw funds for essentially the cost of a uniswap trade.

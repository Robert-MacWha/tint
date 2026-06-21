# Tint Circuits

This package contains tint's circom circuits. These are used for generating inclusion and spendability proofs for the smart contracts.

## Circuits

### Aggregator

The aggregator circuit is tint's primary circuit, and is used to manage the state of the merkle tree. It has two primary jobs:
1. Prove that value is conserved.
2. Prove that pending commitments are correctly graduated into the merkle tree.

**Job 1** ensures the following:
- Each input note's nullifier is correctly derived from its nullifying key and leaf index.
- Each input note's commitment is included in `newRoot`.
- Each output note's commitment is correctly derived from its asset, amount, and partial commitment.
- For each asset, the total input amount equals the total output amount (including unshields).
- Every output asset has at least one matching input asset.

**Job 2** is unique to tint. Unlike most privacy protocols which insert commitments into the merkle tree at deposit time, requiring costly on-chain hashing, tint graduates commitments lazily. When a deposit is made, the commitment is placed in a pending queue rather than the tree. When a user performs a shielded operation, they batch-graduate pending commitments as part of the same proof.

This is proven via two public signals:
- `leavesAggregationHash`: a sequential Poseidon accumulation of the new leaves, chained from `startingAggregationHash`. The contract uses this to verify which commitments were graduated without rehashing them on-chain.
- `newRoot`: the merkle root after the new leaf batch has been inserted at `batchIndex`.

Because inclusion proofs are checked against `newRoot`, newly graduated commitments can be spent in the same transaction they are graduated.

### Spendability Circuits

Spendability circuits are used to privately prove ownership over notes. When notes are created they bind themselves to a particular spendability circuit hash. To spend a note, the user must provide a valid proof for the corresponding spendability circuit. Using spendability circuits, notes can have complex spend conditions beyond simple signature checks, for example:
- timelocks
- multisigs
- vesting schedules
- limit orders

Current spendability circuits include:
- `eddsaPoseidon`: Basic eddsa signature verification
- `eddsaPoseidonMultisig`: Multisig eddsa signature verification

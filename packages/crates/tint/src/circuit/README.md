# Tint Circuit

Tint's zk circuit is implemented in Rust using arkworks. It targets Groth16 / BN254 to prioritize easy proof verification on-chain.

Tint's circuits have two jobs:
1. To verify a private operation.
2. To verify a staging of commitments.

## Operation

An operation is a single private tint "Transaction" that is used to transfer assets. An operation is made of some input notes (which are nullified), some output notes (which are produced), and some unshielded assets (which are withdrawn from the shielded pool). Operations are net-zero, meaning they cannot create or destroy assets.

## Staging

Staging is the process of taking a set of notes and inserting them into the commitment merkle tree. Staging is done in batches.

## Verification

### Operation Verification

An operation is valid when the following conditions are met:
1. The operation is balanced: the sum of assets in equals the sum of assets out.
2. All inputs to the operation exist within the commitment merkle tree.
3. All inputs are nullified with their bound nullifying key.

### Staging Verification
# Spendability

Tint enables assigning arbitrary spendability rules to each note. These rules are enforced by the contracts when a note is spent and can be used to implement features like hardware wallet support, time locks, multi-sigs, p2p swaps, and more.

## Implementation

Each user publishes a public `spendabilityHash` (`poseidon2(spendabilityAddress, spendabilityWitness)`).

### Commit Time

When creating a note for a counterparty, the sender includes their `spendabilityHash` in the note precommitment, binding that note to the spendability rules.

### Operate Time

When a user wishes to spend a note, they will provide the `spendabilityAddress` and a `spendabilityInput` as public inputs and the `spendabilityWitness` as a private input. The circuit will verify that the `spendabilityHash` matches the expected value, then call the `spendabilityAddress` contract with the `spendabilityInput` and remaining operation data. The contract can then verify that the operation is valid according to the spendability rules, reverting if not.

### Signing Hash

Spendability rules should authorize the operation's `signingHash` rather than its `operationHash`:

```
contextHash = keccak256(abi.encode(ciphertexts, unshieldRecipients)) mod p
signingHash = poseidon2(operationHash, contextHash)
```

`operationHash` covers the operation's notes and withdrawal amounts, but not where withdrawals are sent or the output ciphertexts. `signingHash` covers everything except the spendability inputs themselves, so whoever produces the proof can't redirect funds a rule authorized. Rule circuits witness the operation and `contextHash` and recompute `signingHash` in-circuit, keeping it their only operation-specific public input. Contracts compute it with `ProofLib.signingHash`.

### Example
```solidity
contract SignatureSpendability is ISpendability {
    address public signer;

    function spendable(IPrivacyPool.Operation calldata op) external view override {
        // Verify that the operation is valid under this contract's spendability rules.
        // Will nearly universally follow a pattern of verifying some ZK proof(s) so that
        // the `spendabilityWitness` is not revealed, but could also read storage or add
        // public inputs or do other in-solidity checks as needed.

        // publicInputs = [op.hash(), op.nullifiers()]
        // for proof in op.proofs():
        //    require(proof.verify(publicInputs), "invalid proof");
    }
}
```

### Spendability Circuits

Spendability verification will generally involve verifying some ZK proof(s) to ensure `spendabilityWitness` is not revealed.

#### Secret Key

Requires the right `privateKey` to spend the note.

```
public:
    field spendabilityHash
    field signingHash
private:
    field spendabilityAddress
    field spendabilityWitness
    field privateKey

assert(spendabilityHash == poseidon2(spendabilityAddress, spendabilityWitness))
assert(privateKey == spendabilityWitness)
```

#### eth_signTypedData_v4

Requires a signature from the right `signer` to spend the note.

```
public:
    field spendabilityHash
    field operationDigest
private:
    field spendabilityAddress
    field spendabilityWitness
    field signer
    field signature[:65]

assert(spendabilityHash == poseidon2(spendabilityAddress, spendabilityWitness))
assert(signer == spendabilityWitness)

assert(recoverSigner(signature, operationDigest) == signer)
```

#### Time Lock

Requires the current timestamp be greater than or equal to the `unlockTimestamp` to spend the note.

```
public:
    field spendabilityHash
    field signingHash
    field operationTimestamp
private:
    field spendabilityAddress
    field spendabilityWitness
    field unlockTimestamp

assert(spendabilityHash == poseidon2(spendabilityAddress, spendabilityWitness))
assert(spendabilityWitness == unlockTimestamp)
assert(operationTimestamp >= unlockTimestamp)
```

#### Limit Order

Requires the current timestamp be less than or equal to the `expirationTimestamp` and that another note in the same transaction is sending the correct amount and asset to the correct recipient.

```
public:
    field spendabilityHash
    field counterpartyNoteHash
    field operationTimestamp
private:
    field spendabilityAddress
    field spendabilityWitness
    field expirationTimestamp
    field expectedCounterpartyNoteHash

assert(spendabilityHash == poseidon2(spendabilityAddress, spendabilityWitness))
assert(spendabilityWitness == poseidon2(expirationTimestamp, expectedCounterpartyNoteHash))
assert(operationTimestamp <= expirationTimestamp)
assert(counterpartyNoteHash == expectedCounterpartyNoteHash)
```

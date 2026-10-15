# Policy

Tint enables assigning arbitrary policy rules to each note. These rules are enforced by the contracts when a note is spent and can be used to implement features like hardware wallet support, time locks, multi-sigs, p2p swaps, and more.

## Implementation

Each user publishes a public `policyHash` (`poseidon2(policyAddress, policyWitness)`).

### Commit Time

When creating a note for a counterparty, the sender includes their `policyHash` in the note precommitment, binding that note to the policy rules.

### Operate Time

When a user wishes to spend a note, they will provide the `policyAddress` and a `policyInput` as public inputs and the `policyWitness` as a private input. The circuit will verify that the `policyHash` matches the expected value, then call the `policyAddress` contract with the `policyInput` and remaining operation data. The contract can then verify that the operation is valid according to the policy rules, reverting if not.

### Signing Hash

Policy rules should authorize the operation's `signingHash` rather than its `operationHash`:

```
contextHash = keccak256(abi.encode(ciphertexts)) mod p
signingHash = poseidon2(operationHash, contextHash)
```

`operationHash` covers the operation's notes and withdrawals (asset, amount, and recipient), but not the output ciphertexts. `signingHash` covers everything except the policy inputs themselves, so whoever produces the proof can't redirect funds a rule authorized. Rule circuits witness the operation and `contextHash` and recompute `signingHash` in-circuit, keeping it their only operation-specific public input. Contracts compute it with `ProofLib.signingHash`.

### Example
```solidity
contract SignaturePolicy is IPolicy {
    address public signer;

    function requireAuthorized(IPrivacyPool.Operation calldata op) external view override {
        // Verify that the operation is valid under this contract's policy rules.
        // Will nearly universally follow a pattern of verifying some ZK proof(s) so that
        // the `policyWitness` is not revealed, but could also read storage or add
        // public inputs or do other in-solidity checks as needed.

        // publicInputs = [op.hash(), op.nullifiers()]
        // for proof in op.proofs():
        //    require(proof.verify(publicInputs), "invalid proof");
    }
}
```

### Policy Circuits

Policy verification will generally involve verifying some ZK proof(s) to ensure `policyWitness` is not revealed.

#### Secret Key

Requires the right `privateKey` to spend the note.

```
public:
    field policyHash
    field signingHash
private:
    field policyAddress
    field policyWitness
    field privateKey

assert(policyHash == poseidon2(policyAddress, policyWitness))
assert(privateKey == policyWitness)
```

#### eth_signTypedData_v4

Requires a signature from the right `signer` to spend the note.

```
public:
    field policyHash
    field operationDigest
private:
    field policyAddress
    field policyWitness
    field signer
    field signature[:65]

assert(policyHash == poseidon2(policyAddress, policyWitness))
assert(signer == policyWitness)

assert(recoverSigner(signature, operationDigest) == signer)
```

#### Time Lock

Requires the current timestamp be greater than or equal to the `unlockTimestamp` to spend the note.

```
public:
    field policyHash
    field signingHash
    field operationTimestamp
private:
    field policyAddress
    field policyWitness
    field unlockTimestamp

assert(policyHash == poseidon2(policyAddress, policyWitness))
assert(policyWitness == unlockTimestamp)
assert(operationTimestamp >= unlockTimestamp)
```

#### Limit Order

Requires the current timestamp be less than or equal to the `expirationTimestamp` and that another note in the same transaction is sending the correct amount and asset to the correct recipient.

```
public:
    field policyHash
    field counterpartyNoteHash
    field operationTimestamp
private:
    field policyAddress
    field policyWitness
    field expirationTimestamp
    field expectedCounterpartyNoteHash

assert(policyHash == poseidon2(policyAddress, policyWitness))
assert(policyWitness == poseidon2(expirationTimestamp, expectedCounterpartyNoteHash))
assert(operationTimestamp <= expirationTimestamp)
assert(counterpartyNoteHash == expectedCounterpartyNoteHash)
```

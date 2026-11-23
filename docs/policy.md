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
contextHash = keccak256(abi.encode(caller, binding, ciphertexts)) mod p
signingHash = poseidon2(operationHash, contextHash)
```

`operationHash` covers the operation's notes and withdrawals (asset, amount, and recipient), but not the output ciphertexts or the context's `caller` and `binding`, which `contextHash` covers. `signingHash` covers everything except the policy inputs themselves, so whoever produces the proof can't redirect funds a rule authorized. Rule circuits witness the operation and `contextHash` and recompute `signingHash` in-circuit, keeping it their only operation-specific public input. Contracts compute it with `ProofLib.signingHash`.

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

Requires a signature from the right `signer` to spend the note. Implemented by `HardwareWalletPolicy` (see [tint-multisig-policy](../packages/crates/tint-multisig-policy/README.md#hardware-wallet-policy)).

The signer signs the EIP-712 message `TintOperation(bytes32 signingHash)`. Since `signingHash` is public, the contract computes the digest itself and passes it in, split into 128-bit halves, rather than the circuit computing keccak. The circuit still recomputes `signingHash` from the witnessed operation, so the digest is bound to the operation.

```
public:
    field policyAddress
    field signingHash
    field digestHi, digestLo  // contract: eip712Digest(TintOperation(signingHash))
private:
    operation, contextHash
    point pubKey
    field signature[r, s]

assert(signingHash == poseidon2(operation.hash(), contextHash))
for input in operation.inputs where input.policyAddress == policyAddress:
    assert(input.policyWitness == pubKeyHash(pubKey))

assert(ecdsaVerify(pubKey, digestHi * 2^128 + digestLo, signature))
```

Hardware wallets show the domain and `signingHash` rather than the operation's amounts and recipients: displaying those would mean keccak-hashing the private note fields in-circuit.

#### Time Lock

Requires the current timestamp be greater than or equal to the `unlockTimestamp` to spend the note. Implemented by [`tint-timelock-policy`](../packages/crates/tint-timelock-policy) and `TimelockPolicy.sol`.

```
public:
    field policyAddress
    field signingHash
    field bound
private:
    field policyWitness   // per input bound to policyAddress
    field unlockTimestamp
    field salt

assert(policyWitness == poseidon2(unlockTimestamp, salt))
assert(unlockTimestamp < 2^64)
assert(bound - unlockTimestamp < 2^64)   // unlockTimestamp <= bound
```

The policy input is `abi.encode(proof, bound)` and the contract requires `bound <= block.timestamp`. Proving against a public `bound` rather than the timestamp lets the prover reveal a coarse bound (e.g. the next whole day) instead of the exact unlock time. The `salt` hides the unlock time from anyone holding the account's policy hash. The contract verifies the proof before checking `bound`, so a placeholder input spends a valid input's gas before reverting.

A timelock only adds a restriction: spending still needs the note's nullifying key. Since each note has one policy, it can't be combined with another rule such as a multisig.

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

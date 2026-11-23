# Intents

Intents let a user post a shielded offer ("I give `a` of asset A for `x` of asset B") that any counterparty can fill without either side leaving the pool. Offers are authorized by a spending policy (`IntentPolicy`) and discovered through an off-chain HTTP relay, so posting and cancelling cost no gas.

## Problem

Spending a note needs its nullifying key, which the join-split witnesses. A maker filling an intent must spend the user's offered note, so the user must hand over spend material without exposing their other notes, and without letting the maker take the funds without paying.

## Design

### Intent note

The user moves the offered funds into a fresh, one-time **intent note**:

- Its nullifying key `nk` is random and published in the intent. It is unrelated to the user's account, so it reveals nothing about the user's other notes.
- Its policy is `IntentPolicy`, with `policyWitness = poseidon2(wantHash, expiry, cancelHash)`.

Since `nk` is public, the policy is the only thing protecting the note. It authorizes an operation spending the note only if either:

- **Fill:** some output of the same operation is exactly the note the user wants, `wantHash`, and the operation's `deadline <= expiry`.
- **Cancel:** the prover knows `cancelKey`, where `cancelHash = poseidon2(poseidon2(cancelKey))`.

### Wanted note

`wantHash` is the full commitment of the note the user receives: `Commitment(wantAsset, wantAmount, receiverPolicyHash, receiverNullifierPubKey, random)`, with `random` drawn by the user.

The amount is exact rather than a minimum. The user therefore knows the whole commitment ahead of time, and finds the received note from the intent alone, by matching `wantHash` against `Committed` events (`Intent::received`). Any price improvement goes to the maker, as with a plain limit order.

Since the user already knows it, a fill creates `want` as an **exact output** (`OperationBuilder::output_note`) that isn't encrypted to anyone: it carries a throwaway-sealed ciphertext of the usual length (see [note encryption](./note-encryption.md#scheme)). Exact outputs don't count toward an operation's single receiver, so one operation can create several `want` notes for different owners, and its ciphertexts look like any other operation's with the same output count. The flip side is that ordinary syncing doesn't find the note: the owner's wallet must keep its open intents, which it needs anyway to cancel them.

The intent publishes the receiver's nullifier public key and policy hash (inside `want`), but not its encryption key, which fills don't need. Owners should receive into a fresh account per intent, since a reused account links its intents to each other and to anyone who knows that account.

### Policy circuit

The circuit follows the [policy pattern](./policy.md#signing-hash): it witnesses the operation and `contextHash`, and recomputes `signingHash` in-circuit.

```
public:
    field policyAddress
    field signingHash
    field slot          // input slot this proof authorizes
    field deadline
private:
    operation, contextHash
    field wantHash, expiry, cancelHash, cancelKey

assert(signingHash == poseidon2(operation.hash(), contextHash))
assert(operation.inputs[slot].policyAddress == policyAddress)
assert(operation.inputs[slot].policyWitness == poseidon2(wantHash, expiry, cancelHash))

fill   = wantHash != 0 && any(output.hash() == wantHash) && deadline <= expiry
cancel = poseidon2(poseidon2(cancelKey)) == cancelHash
assert(fill || cancel)
```

Each intent input gets its own proof, selected by `slot`, so one operation can spend several intents with different terms. This is what lets a third party match two users' intents against each other.

### Contract

`IntentPolicy.requireAuthorized` runs once per operation. For each input slot bound to it, it decodes `policyInputs[slot] = abi.encode(Proof, uint64 deadline)`, requires `block.timestamp <= deadline`, and verifies the proof against `[address(this), signingHash, slot, deadline]`.

The prover picks `deadline`, so it reveals only a bound no later than `expiry`. Cancels use `deadline = type(uint64).max`.

### Intent format

An `Intent` (Rust, serde) holds everything a filler needs:

| field | meaning |
| --- | --- |
| `policy_address` | the `IntentPolicy` deployment |
| `offer` | the intent note's commitment |
| `nullifier_key` | the intent note's one-time nullifying key |
| `want` | the commitment a fill must create |
| `expiry` | unix time after which fills are refused |
| `cancel_hash` | `poseidon2(poseidon2(cancelKey))` |

A filler checks the intent is consistent (the offer's nullifier key and policy hash match the terms), that the offer is in the note set and its nullifier unspent, then builds an operation that spends `offer` and outputs `want`.

## Flows

**Post.** The owner builds an `Intent`, keeps `cancelKey`, funds the intent note with an ordinary operation (an exact output of the intent note's commitment), and posts the intent to the relay.

**Limit order.** A maker spends the intent note plus its own notes of the wanted asset, outputs `want`, and takes the offered asset and its change.

**P2P swap.** Two owners post opposite intents. Anyone can spend both intent notes in one operation, outputting both `want` notes as exact outputs. Any surplus goes to the matcher as change.

**Cancel.** The owner spends the intent note back to themselves with the cancel branch, and deletes it from the relay. Expired intents can't be filled, but still need the cancel branch to reclaim.

## Relay

A minimal HTTP JSON service with an in-memory store:

- `POST /intents` posts an intent, keyed by its offer commitment.
- `GET /intents?offer_asset=..&want_asset=..` lists unexpired intents.
- `DELETE /intents/{id}` with `{"cancel_token": poseidon2(cancelKey)}` removes one. The relay checks `poseidon2(cancel_token) == cancel_hash`; the token doesn't let the relay spend the note.

The relay is untrusted and advisory: it doesn't check intents against the chain, and deleting an intent doesn't stop anyone who already has it from filling it. Only the on-chain cancel does.

## Privacy

The intent is public, so the operation that funds the intent note and the one that fills or cancels it are linkable to the intent (anyone can compute the offer's nullifier). The owner's other inputs, the maker's notes, and the later spend of the received note (whose nullifier needs the receiver's key) stay hidden.

## Not supported

- **Partial fills.** A fill must take the whole offer. Supporting them would need the circuit to accept a change output back into a new intent note with proportionally reduced terms.
- **Cross-chain intents.** The format is single-chain; a cross-chain fill needs a bridge or settlement layer to prove the counterpart output on another chain.
- **Relay authentication, persistence, or spam control.**

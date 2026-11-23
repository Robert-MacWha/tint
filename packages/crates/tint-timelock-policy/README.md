# tint-timelock-policy

A Tint policy rule that stops a note from being spent before an unlock time.

> [!IMPORTANT]
> This is a proof of concept. It contains unaudited code and uses a dev trusted setup.

## Example

`TimelockPolicy::authorize` proves the unlock time is at most `bound` and attaches the proof to an operation.

```rust
let timelock = TimelockPolicy::new(contract, unlock_time, salt, artifacts);
let account = Account::from_seed(&seed, timelock.policy());

// ...later, once `bound <= block.timestamp`
timelock.authorize(&mut op, bound)?;
let proven = op.prove_with_mmr(&mmr, &join_split_artifacts, &mut rng)?;
```

See [`tests/timelock.rs`](tests/timelock.rs) for a full deposit and spend against anvil.

## Design

Notes commit to `policyWitness = poseidon2(unlockTime, salt)`. The salt stops anyone holding the account's policy hash from brute forcing the unlock time.

The circuit proves `unlockTime <= bound` for a public `bound`, and [`TimelockPolicy.sol`](../../contracts/src/policy/timelock/TimelockPolicy.sol) checks `bound <= block.timestamp`. The prover picks the bound, so it can reveal a coarse value like the next whole day instead of the exact unlock time. Both `unlockTime` and `bound - unlockTime` are range checked to 64 bits.

Like the other policy circuits, the proof recomputes the operation's `signingHash` in-circuit. It can't be replayed on another operation.

The timelock only adds a restriction. Spending still needs the note's nullifying key, which the join-split checks.

## Limitations

A note has one policy, so a timelock can't be combined with another rule such as a multisig.

## Commands

```bash
just build-solidity                            # regenerate src/codegen/TimelockVerifier.sol
cargo test --release -- --ignored              # end-to-end test against anvil
forge script script/DeployTimelockPolicy.s.sol --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast
```

Run the `forge` command from `packages/contracts/`.

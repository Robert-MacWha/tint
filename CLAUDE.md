# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Tint is an EVM-focused, UTXO-based privacy protocol proof-of-concept. Notes are shielded ERC20 deposits; transfers/unshields are proven with Groth16 zk-SNARKs (BN254) that verify note ownership and policy without revealing amounts or parties. See `README.md` for the protocol pitch and gas-cost rationale, and `docs/note-lifecycle.md` / `docs/policy.md` for the two core design ideas (a skew-MMR set commitment with one-hash appends, and pluggable per-note policy rules).

## Repo layout

Monorepo split into two independently-built packages:

- `packages/contracts/` — Foundry/Solidity. `Tint.sol` is the main privacy pool contract; it holds the note set in a skew-MMR (`LibSkewMmr` from the `efficient-inclusion-proofs` submodule, via its `SkewMmrWithHistory` contract, which Tint inherits for the frontier-commitment ring). `src/codegen/` holds the **generated** Groth16 verifiers (see below) — don't hand-edit them.
- `packages/crates/` — Rust workspace (members: `tint`, `cli`, `circuit-profiler`).
  - `tint/` — the core protocol crate: circuits (`src/circuit/`, arkworks + Groth16/BN254), accounts and key derivation (`src/account/`), spending policies (`src/policy.rs`), notes/commitments (`src/note/`), note encryption (`src/crypto/`), event sync into serializable `Snapshot`s (`src/syncer/`), the skew-MMR (`src/skew_mmr.rs`), the `Deposit` builder (`src/deposit.rs`), the `OperationBuilder` → `UnsignedOperation` → `Operation` → `ProvenOperation` flow that assembles transfer/unshield calls + proofs; `Operation` is also the padded witness policy circuits consume (`src/operation.rs`), and `TintProviderExt` accessors for any alloy provider (`src/provider_ext.rs`). Note management is left to the caller. `src/codegen.rs` emits the Solidity verifier from a Groth16 `VerifyingKey`.
  - `cli/` — a minimal demo CLI (`tint-cli`) wrapping the `tint` crate; `src/chain.rs` handles RPC/tx flow, `src/config.rs` handles local account storage.
  - `tint-aml/` — separate circuit crate (AML-related), not part of the main protocol flow.


## Commands

Use `nix develop` (or direnv, via `.envrc`) to get `just`, `pnpm`, `foundry`, and the pinned Rust toolchain.

```bash
just run <cli-args>       # builds contracts (forge build) then `cargo run --release` for tint-cli
just setup                 # forge build only (contracts)
just env                   # prints RPC_URL/PRIVATE_KEY etc. decrypted from secrets/secrets.yaml via sops
```

Rust (run from `packages/crates/`):
```bash
cargo build
cargo test                                  # unit tests + fast integration tests
cargo test --release -- --ignored           # full integration tests (spin up anvil, deploy contracts, generate real Groth16 proofs) — slow, release-only
cargo clippy                                 # pedantic lints are enabled workspace-wide (warn)
cargo run --release --bin gen_artifacts      # regenerate proving/verifying keys + constraint matrices under tint/artifacts/ after a circuit change
cargo run --bin gen_verifier                 # regenerate packages/contracts/src/Groth16Verifier.sol from the current VerifyingKey
```
Integration tests under `packages/crates/tint/tests/` are `#[ignore]`d because they deploy a full Anvil instance and run real proof generation; the `common/anvil.rs` helper sets that up.

Solidity (run from `packages/contracts/`, or via CI's exact invocation):
```bash
forge fmt --check
forge build --sizes
forge test -vvv
```

## Working across the circuit/contract boundary

The circuit (`tint/src/circuit/join_split.rs`) and `Tint.sol` must agree on public input layout and constants (`N_INPUTS`, `N_OUTPUTS`, `N_WITHDRAWALS`, `N_PUB`, `MMR_MAX_DEPTH`, etc. — see `packages/contracts/src/lib/Constants.sol` and `tint/src/circuit/join_split.rs`).

The public-signal vector is built by `ProofLib.toPublicSignals` and must match the order `JoinSplitResultVar::flatten()` allocates in:

| index | signal |
| --- | --- |
| 0 | `boundParamsHash` |
| 1 | `operationHash` |
| 2 | `histRanks` — packed tree ranks, 6 bits each |
| 3 | `histDepth` — number of live trees |
| 4 | `histCount` — number of elements appended |
| 5 .. | `nullifier[i]`, `policyAddress[i]` for `N_INPUTS` |
| .. | `commitmentOut[i]` for `N_OUTPUTS` |
| .. | `unshieldAmount[i]`, `unshieldAsset[i]`, `unshieldRecipient[i]` for `N_WITHDRAWALS` |
| `FRONTIER_OFFSET` .. | frontier roots, zero-padded to `MMR_MAX_DEPTH = 42` |

`Operation.pub` carries only the live prefix (`FRONTIER_OFFSET + histDepth` words) as calldata; the contract zero-pads it back to `N_PUB`, and checks `pub[0]` against the hash recomputed from `context`.

Hybrid compression folds all `N_PUB` signals down to the 3 the Groth16 verifier checks, so adding a signal costs in-circuit witnessing rather than pairing gas.

Whenever the circuit's shape or logic changes:
1. Regenerate artifacts with `gen_artifacts`.
2. Regenerate the on-chain verifier with `gen_verifier`.
3. Contract-side constants in `Constants.sol` may need manual updates to stay in sync, as do the mirrored constants in the inline `sol!` block in `tint/src/abis/tint.rs`.

`tint/tests/public_signals_match_onchain.rs` and `tint-poseidon2/tests/poseidon2_match_onchain.rs` exist specifically to catch drift between the Rust circuit and Solidity contracts — run these after touching either side.

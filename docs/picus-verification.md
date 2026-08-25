# Picus / arkworks interoperability

[Picus](https://github.com/chyanju/Picus) is a formal verification tool that checks whether an R1CS circuit is "well-constrained": given the circuit's declared inputs, is every declared output uniquely determined, or can a malicious prover satisfy the same constraints with a different output (an under-constrained-signal bug)? It reads the iden3/circom binary `.r1cs` format.

Tint's circuits are written in arkworks (`packages/crates/tint/src/circuit/`), not circom, and exported to `.r1cs` via `to_r1cs_writer` in `circuit/matrices.rs`. Getting that export right is non-obvious — arkworks' R1CS model is a poor match for the assumptions circom-derived tooling makes, and a naive export produces a `.r1cs` file that Picus will happily accept and report `safe` on without actually checking anything. This doc records what's been learned so the next person (or the next session) doesn't have to rediscover it.

## The core mismatch

Circom's R1CS format assumes a circuit author explicitly declares, per signal, one of four categories, laid out in the wire vector as `[const, pub_out, pub_in, prv_in, intermediate]`:

- **`pub_out`** — values the circuit *computes* and exposes publicly (e.g. a hash the circuit derives and asserts equal to an external value).
- **`pub_in`** / **`prv_in`** — values *given* to the circuit (public or private), never computed.
- **intermediate** (uncounted, uses the rest of the wire range) — every other signal, mostly internal gadget wiring.

Arkworks has none of this. It has exactly two variable classes — "instance" (public) and "witness" (private) — with no way to ask "was this computed or given?" after the fact. `ConstraintSystemRef` doesn't track it, and the distinction has to be reconstructed by the exporter using out-of-band knowledge about how the circuit was written.

Picus's actual checking algorithm (`crates/picus-analysis/src/dpvl.rs` in the Picus repo) makes this concrete:

```rust
let input_set: HashSet<usize> = r1cs.inputs.iter().copied().collect();   // = pub_in ∪ prv_in ∪ {const}
let output_set: HashSet<usize> = r1cs.outputs.iter().copied().collect(); // = pub_out only
let target_set = output_set;
let mut ks = input_set.clone();  // "known" (given, never checked)
```

Two consequences fall directly out of this:

1. **Only `pub_out` wires are ever checked.** `pub_in` and `prv_in` are assumed correct by fiat. If `target_set` is empty (`n_pub_out == 0`), the very first loop iteration returns `Safe` — `HashSet::all()` on an empty set is vacuously true — without running propagation or the solver at all. A `.r1cs` file with `n_pub_out = 0` will *always* report `safe`, regardless of what the constraints actually say.
2. **`prv_in` and "intermediate" are not the same thing**, even though arkworks conflates them (both are just "witness"). Only wires *outside* `1 + n_pub_out + n_pub_in + n_prv_in` are left as "unknown" for Picus to actually reason about. If the exporter reports every witness variable as `n_prv_in`, there is no unknown internal state left for the algorithm to find a counterexample in — every output resolves via trivial forward substitution from an already-fully-known witness, and the check is vacuous again, just less obviously so (it can look like real analysis ran, since propagation lemmas fire and the CLI takes nonzero time).

## What `to_r1cs_writer` has to get right

### 1. Public wires: `n_pub_out` vs `n_pub_in`, and a wire permutation

Tint's circuit helpers (`circuit/mod.rs`) already draw the right distinction at the Rust level: `input()` allocates a genuine given public value, `output()` allocates a public value *and* enforces it equal to something the circuit computed. Both are `T::new_input` under the hood — arkworks makes no distinction — but the call site tells you which is which.

The first version of the exporter reported every public wire as `n_pub_in` (with `n_pub_out` hardcoded to `0`). This made every check on every circuit vacuously `safe` per point 1 above.

The fix requires two things, not just relabeling the header counts:

- Track how many of a circuit's public wires are true `input()`s (`n_pub_in`), passed explicitly by the caller. For `JoinSplit`, that's `4` (`old_root`, `start_aggregation_index`, `start_aggregation_hash`, `bound_params_hash` — all `input()`, allocated before any `output()` call in `JoinSplit::synthesize`).
- **Permute the public wire columns.** Arkworks lays public wires out in allocation order — for `JoinSplit`, `input()`s first (wires 1-4), then `output()`s (wires 5-26). The R1CS format expects the *opposite* physical order: outputs immediately after the constant wire, inputs after that. Just swapping the header's `n_pub_out`/`n_pub_in` counts without also swapping the wire columns silently mislabels wires — the *first* `n_pub_out` physical positions get called "outputs" regardless of what's actually there. `to_r1cs_writer` remaps every constraint's column indices to swap the two blocks.

This relies on an invariant that isn't checked anywhere: **a circuit must allocate all its true `input()`s before any `output()`s.** True today for `JoinSplit`; nothing enforces it stays true.

### 2. Private wires: `n_prv_in` vs. intermediate

Even after fixing (1), a `JoinSplit` check reported `safe` in a single propagation pass with zero solver calls — a red flag, since a check that never touches the solver on a circuit this size isn't testing anything interesting. The reason: `n_prv_in` was `matrices.num_witness_variables` (67,031) — *every* witness variable, including every internal Poseidon2 S-box helper and Merkle-step intermediate hash, not just the circuit's genuinely-declared private inputs.

The fix here doesn't need a permutation — arkworks already allocates a circuit's declared private inputs before any internal gadget wiring (for `JoinSplit`: the single `witness(cs.clone(), self)?` call allocates the entire private struct in one shot, *before* `verify()` runs and allocates everything else). It only needs the right *count* to stop at. `JoinSplit::n_declared_private_inputs()` (`circuit/join_split.rs`) gets this by literally re-running just the witness allocation on a throwaway `ConstraintSystem`:

```rust
pub fn n_declared_private_inputs() -> Result<usize, SynthesisError> {
    let cs = ConstraintSystem::new_ref();
    let _: JoinSplitVar = witness(cs.clone(), &JoinSplit::default())?;
    Ok(cs.num_witness_variables())
}
```

This is correct by construction (not a guessed/hardcoded literal) because arkworks' `AllocVar`-derived allocation is value-independent — it has to be, for a single proving key to work regardless of witness values — so `JoinSplit::default()` allocates exactly as many variables as the real witness does. For `JoinSplit` this comes out to `933`; the remaining `67,031 - 933 = 66,098` witness wires are left uncounted, landing in the "intermediate" range Picus treats as genuinely unknown.

### Current `to_r1cs_writer` signature

```rust
pub fn to_r1cs_writer<F: PrimeField>(
    matrices: &Matrices<F>,
    n_pub_in: u32,   // caller-supplied: how many public wires are true input()s
    n_prv_in: u32,   // caller-supplied: how many witness wires are true declared private inputs
) -> ark_r1cs_exporter::R1CSWriter<F>
```

Both counts are manually supplied per export call site (`export_r1cs.rs` passes `4` and `JoinSplit::n_declared_private_inputs()`; `export_picus_sanity.rs` passes `0` and the full witness count, since those toy circuits have no internal/intermediate wires worth separating out). Nothing automatically keeps these in sync with circuit changes — a real gap. If `JoinSplit` ever gains a new `input()`/`output()` in the wrong place, or the private struct's shape changes, these numbers need to be revisited by hand.

## Verifying the fix actually does something

Before trusting any of this, it was validated against two minimal hand-built circuits (`bin/export_picus_sanity.rs`):

- **`Constrained`**: `c = a + b`, fully forced by the witness. Should report `safe`.
- **`Underconstrained`**: `c` exposed directly via `input()` (not routed through a witness first — that would make it trivially "known" and defeat the test), constrained only by `c² = a²`, which both `c = a` and `c = -a` satisfy for the same witness `a`. Should report `unsafe`.

With the pre-fix exporter (`n_pub_out` hardcoded to `0`), **both** reported `safe`, confirming the check was vacuous even for an obviously-broken circuit. With the fix, `Constrained` stayed `safe` and `Underconstrained` correctly flipped to `unsafe`, with Picus producing a genuine counterexample (`c = 1` vs. `c = p-1` for the same witness). This pair is worth re-running after any further change to `to_r1cs_writer` — it's a fast (~seconds) regression check that catches vacuous-check bugs the full `JoinSplit` run is too slow to catch quickly.

## Practical: running Picus on `join_split.r1cs`

The circuit is large (66,742 constraints, 67,058 wires) and the native FF solver can be memory- and time-hungry once internal wires are genuinely exposed (see below). Two things make this tractable to run without surprises:

**Cap memory** so a runaway solve gets killed cleanly instead of taking down the whole machine:
```bash
systemd-run --user --scope -p MemoryMax=16G -- <command>
```

**Turn on debug logging** to see what's actually happening instead of waiting blind for a final `safe`/`unsafe`/`unknown`. `picus-cli` uses `env_logger`; `picus-analysis`'s DPVL loop logs per-signal and per-lemma progress at `debug` level:
```bash
systemd-run --user --scope -p MemoryMax=16G -- env RUST_LOG=debug \
  picus check --r1cs ./packages/crates/tint/artifacts/join_split.r1cs \
  --gb-strategy auto --profile wall --gb-stats --timeout 300000 \
  2>&1 | tee picus_debug.log
```

Useful lines to grep for in the output:
- `lemma <name> fired=.. ks+=..` — per-lemma-per-round propagation progress.
- `propagation round: ks=X, us=Y` — how many of the total wires are pinned down vs. still open after a round. If `us` never drops much below its post-propagation value, the solver is grinding without making progress.
- `Solving signal <id> (target=<bool>)` — which wire the solver is attempting. `target=false` means it's not one of the real outputs — DPVL is trying to resolve an *intermediate* wire on the way toward proving a real target, which is most of where time goes on a circuit this size.
- `solver returned Unknown for wire <id>: <reason>` — a per-wire give-up, with a reason (`Timeout` observed in practice).

The `=== picus split-GB driver stats ===` block (from `--gb-stats`) is useful for a post-mortem: `solve_calls` tells you how many distinct signals the solver actually attempted (as low as `1` before the `n_prv_in` fix — i.e. it gave up on the very first thing it tried and never got further); `solve_inner` is wall time spent inside the GB solve itself.

**Unexplained anomaly**: individual per-signal solve attempts have consistently taken far less time than the configured `--timeout` (e.g. ~15-40s observed against both `--timeout 60000` and `--timeout 300000`, with no meaningful difference between the two), all still reported as reason `Timeout`. Something other than the CLI's `--timeout` flag appears to be capping individual attempts. Not yet root-caused — worth checking `--cdclt-iter-cap` / `--dnf-cap` more carefully, or reading `picus-smt`'s backend solve-loop source directly, before assuming it's a pure wall-clock timeout.

## Why the full circuit is currently intractable, and what to do about it

Once `n_prv_in` correctly excludes internal wires, `join_split.r1cs`'s "unknown" pool jumps from 22 wires (just the real outputs) to roughly 66,000+ (every internal Poseidon2/Merkle-path wire). Propagation resolves a large chunk of these for free (~15,000 of ~67,000 wires in one pass observed), but the remainder falls to the solver, which has to prove each candidate wire's uniqueness via Gröbner basis computation — and Poseidon2's degree-5 S-box makes each of these individually expensive (tens of seconds, frequently timing out) for a general-purpose finite-field GB engine. Observed wire indices during a stuck run incremented in a stride of 3, consistent with grinding through consecutive state elements of the circuit's `T3` (width-3) Poseidon2 permutation.

Brute-forcing through tens of thousands of such wires at ~30s each is not a slow check, it's an intractable one. Two complementary ways forward, discussed but not yet executed:

1. **Isolate a single gadget.** Export just one Poseidon2 `T3` permutation (or one Merkle-inclusion step) as its own tiny circuit and check it alone. Fast and cheap, and answers "is this primitive checkable by this solver at all" independent of the surrounding protocol — but doesn't exercise how the gadget is *wired into* the rest of the circuit.
2. **Shrink `JoinSplit`'s parameters.** `N_INPUTS`/`N_OUTPUTS`/`N_WITHDRAWALS`/`TREE_DEPTH`/aggregation-step-count are compile-time constants today; making them generic and running with small values (e.g. 2 inputs/outputs, depth-2 tree, 8 aggregation steps instead of 64) directly cuts the volume of repeated hash-internal wires the solver has to grind through, while still exercising the real end-to-end protocol wiring (unlike option 1). This is the direction currently planned.

**Important caveat for whichever path is taken**: Picus checks one concrete R1CS instance, not a parameterized family. Proving a reduced-parameter (`N=2`) instance safe is strong evidence — by structural symmetry, since each nullifier/withdrawal/Merkle-step/aggregation-step is an independent, structurally identical copy of the same sub-circuit — that the full-size (`N=5`) production instance is also safe, but it is not a formal proof of the `N=5` case unless the full-size circuit is eventually also checked (or every code path is confirmed to be exercised identically regardless of `N`, e.g. no first/last-iteration special-casing that a small `N` wouldn't hit). Don't round this up to "formally verified" in write-ups or commit messages — say what was actually checked.

## Nix packaging notes

`nix/picus/package.nix` builds `picus-cli` natively (no `cvc5`/`z3` features — MIT-licensed, no extra system deps, matches `docs/building.md`'s documented default build). Two build issues hit during setup, both fixed in the package derivation:

- `gmp-mpfr-sys` (a transitive dependency of `rug`, used by `picus-core`/`picus-solver`) builds GMP/MPFR from vendored source via autotools and needs `m4` on `PATH` — added to `nativeBuildInputs`.
- `cargo test`'s default `doCheck` picks up `crates/picus`'s `r1cs_smoke` integration test, which needs a `circomlib` git submodule and circom-compiled circuits unavailable in the sandbox. Skipped via `PICUS_SKIP_PLDI_SMOKE=1`, the escape hatch the test's own panic message documents.
- `pkgs.callPackage` auto-wires `cvc5 ? null` / `z3 ? null` function args from `pkgs.cvc5`/`pkgs.z3` if those attributes exist in nixpkgs, regardless of the `? null` default — silently enabling both optional backends (and their ~15-20 min from-source builds) unless the call site explicitly passes `cvc5 = null; z3 = null;`. Not an issue for the current derivation (those args were removed once cvc5/z3 support was stripped out entirely), but worth remembering if optional-package args are reintroduced anywhere in this flake.

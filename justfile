# Tint circuit workflows. Run `just` to list recipes.

default:
    @just --list

# Run circuit unit tests. Pass args through, e.g. `just test --package tint_aggregator`.
test *ARGS:
    nargo test --workspace {{ ARGS }}

# Compile every circuit in the workspace.
compile:
    nargo compile --workspace --silence-warnings

# Format Noir sources.
fmt:
    nargo fmt

# Benchmark every circuit (ACIR/gate counts, proving time, proof/VK sizes).
bench:
    ./scripts/bench.sh

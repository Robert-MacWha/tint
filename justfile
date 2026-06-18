# Tint circuit workflows. Run `just` to list recipes.

default:
    @just --list

# Run circuit unit tests.
test:
    nargo test --workspace

# Compile every circuit in the workspace.
compile:
    nargo compile --workspace

# Format Noir sources.
fmt:
    nargo fmt

# Benchmark every circuit (ACIR/gate counts, proving time, proof/VK sizes).
bench:
    ./scripts/bench.sh

# Prove a minimal 4-circuit Chonk folding stack (PoC).
chonk-poc:
    cd scripts/chonk_poc && pnpm install && pnpm start

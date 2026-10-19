set export := true

TINT_ADDRESS := "0x8F5DA18CbB981c809d95E01d0aBC1a17B5B29B05"
PASSWORD_POLICY_ADDRESS := "0xe78836929dc9cfbbee7c4d262d7721b43e4848dd"
MULTISIG_POLICY_ADDRESS := "0x3e597E5aD27891eae319DBBBbAe71a3b0e9aCEd7"
TOKEN := "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"

env:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TINT_ADDRESS=${TINT_ADDRESS}"
    echo "PASSWORD_POLICY_ADDRESS=${PASSWORD_POLICY_ADDRESS}"
    echo "MULTISIG_POLICY_ADDRESS=${MULTISIG_POLICY_ADDRESS}"
    echo "TOKEN=${TOKEN}"
    if [ -f secrets/secrets.yaml ]; then
        sops -d --output-type dotenv secrets/secrets.yaml
    else
        echo "secrets/secrets.yaml not found, skipping secrets" >&2
    fi

build:
    just build-artifacts
    just build-solidity

    cd packages/contracts && forge soldeer install && forge build
    cd packages/crates && cargo build --release

build-artifacts:
    cd packages/crates/tint && just build-artifacts
    cd packages/crates/tint-password-policy && just build-artifacts
    cd packages/crates/tint-multisig-policy && just build-artifacts

build-solidity:
    cd packages/crates/tint && just build-solidity
    cd packages/crates/tint-password-policy && just build-solidity
    cd packages/crates/tint-multisig-policy && just build-solidity

run *ARGS:
    set -euo pipefail
    eval "$(just env)"
    cd packages/crates/cli && cargo run --release -- {{ARGS}}

# Builds the tint worker and serves the web UI with `dx serve`.
web:
    #!/usr/bin/env bash
    set -euo pipefail
    # The worker embeds the contract ABIs to decode reverts.
    (cd packages/contracts && forge build)
    cd packages/crates
    # Proving runs on a rayon thread pool (wasm-bindgen-rayon), which needs std
    # rebuilt with atomics, an unstable cargo feature. rustc doesn't link a
    # shared memory or export the TLS/heap symbols wasm-bindgen's thread
    # support needs on its own, so pass them, with the memory's max at the
    # wasm32 limit.
    RUSTC_BOOTSTRAP=1 \
    RUSTFLAGS="-C target-feature=+atomics,+bulk-memory \
        -C link-arg=--shared-memory -C link-arg=--import-memory \
        -C link-arg=--max-memory=4294967296 \
        -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size \
        -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base \
        -C link-arg=--export=__heap_base" \
        cargo build --release -p tint-web-worker --target wasm32-unknown-unknown \
        -Z build-std=panic_abort,std
    wasm-bindgen --target web --out-dir tint-web/public/worker \
        target/wasm32-unknown-unknown/release/tint_web_worker.wasm
    mkdir -p tint-web/public/artifacts
    # Hard links, since dx doesn't copy symlinks into its build.
    for f in proving_key matrices verifying_key; do
        ln -f "tint/artifacts/$f.bin.br" "tint-web/public/artifacts/$f.bin"
    done
    # Shared wasm memory needs the page to be cross-origin isolated.
    cd tint-web && dx serve --cross-origin-policy

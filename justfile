set export := true

TINT_ADDRESS := "0x9e86ed840D1b97E047718318eE96af15F16E5BeD"
PAYMASTER_ADDRESS := "0xcd3d4FEc298436676B40a889AfcFAC5bf3111c4C"
PASSWORD_POLICY_ADDRESS := "0xe78836929dc9cfbbee7c4d262d7721b43e4848dd"
MULTISIG_POLICY_ADDRESS := "0x3e597E5aD27891eae319DBBBbAe71a3b0e9aCEd7"
TOKEN := "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"

env:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TINT_ADDRESS=${TINT_ADDRESS}"
    echo "PAYMASTER_ADDRESS=${PAYMASTER_ADDRESS}"
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

# Runs a local alto bundler against a Tenderly virtual testnet at $RPC_URL, for
# tint-web's sponsored operations. TintPaymaster breaks the ERC-7562 rules, so
# alto runs without safe mode.
bundler:
    #!/usr/bin/env bash
    set -euo pipefail
    eval "$(just env)"
    # Throwaway keys, funded with Tenderly's cheatcode.
    executor=0x4a3a02862ddcb260ed52d40ef03f8e3d78fa3d174b0ef333afdf1ffb4a648cd5
    utility=0xdd4b2564c83ff7de602c39ffda1146055dc1814b07c083d7971722384f1f01a6
    for key in $executor $utility; do
        cast rpc tenderly_setBalance "$(cast wallet address $key)" 0x3635C9ADC5DEA00000 \
            --rpc-url "$RPC_URL" > /dev/null
    done
    alto --rpc-url "$RPC_URL" \
        --entrypoints 0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108 \
        --executor-private-keys $executor --utility-private-key $utility \
        --port 4337 --safe-mode false --enable-cors true

# Builds the tint worker into tint-web/public, alongside the proving
# artifacts it serves.
web-worker:
    #!/usr/bin/env bash
    set -euo pipefail
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
    # Rust's default wasm features, plus threads for the shared memory.
    wasm-opt -O3 --enable-threads --enable-bulk-memory --enable-mutable-globals \
        --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types \
        --enable-multivalue tint-web/public/worker/tint_web_worker_bg.wasm \
        -o tint-web/public/worker/tint_web_worker_bg.wasm
    mkdir -p tint-web/public/artifacts
    # Hard links, since dx doesn't copy symlinks into its build.
    for f in proving_key matrices verifying_key; do
        ln -f "tint/artifacts/$f.bin.br" "tint-web/public/artifacts/$f.bin"
    done

# Serves the web UI with `dx serve`. tint-web is built for the deployment
# `just env` describes, sending sponsored operations to BUNDLER_URL.
web BUNDLER_URL="http://127.0.0.1:4337":
    #!/usr/bin/env bash
    set -euo pipefail
    just web-worker
    set -a; eval "$(just env)"; set +a
    # tint-web links transactions to $EXPLORER_URL/tx/HASH.
    export EXPLORER_URL="${EXPLORER_URL%/transactions}"
    # Shared wasm memory needs the page to be cross-origin isolated.
    cd packages/crates/tint-web && dx serve --cross-origin-policy

# Builds the web UI into packages/crates/tint-web/dist, to serve with
# deploy/Caddyfile. BUNDLER_URL is the bundler as the browser reaches it,
# e.g. https://tint.example.com/bundler.
web-build BUNDLER_URL:
    #!/usr/bin/env bash
    set -euo pipefail
    just web-worker
    set -a; eval "$(just env)"; set +a
    export EXPLORER_URL="${EXPLORER_URL%/transactions}"
    cd packages/crates/tint-web
    # dx stages bundles here and never clears out old hashed assets.
    rm -rf dist ../target/dx/tint-web/release/web
    # Debug symbols crash wasm-opt.
    dx bundle --web --release --debug-symbols false --out-dir dist
    # Caddy serves these precompressed; the artifacts shrink the most.
    find dist/public -type f \( -name '*.bin' -o -name '*.wasm' -o -name '*.js' \) \
        -exec gzip -k -9 {} +

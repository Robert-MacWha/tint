set export := true

TINT_ADDRESS := "0xc9b1a7861dccf2b7c573d8379958b09972fa7053"
TOKEN := "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"

env:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TINT_ADDRESS=${TINT_ADDRESS}"
    echo "TOKEN=${TOKEN}"
    if [ -f secrets/secrets.yaml ]; then
        sops -d --output-type dotenv secrets/secrets.yaml
    else
        echo "secrets/secrets.yaml not found, skipping secrets" >&2
    fi

setup:
    cd packages/contracts && forge build

run *ARGS: setup
    set -euo pipefail
    eval "$(just env)"
    cd packages/crates/cli && cargo run --release -- {{ARGS}}
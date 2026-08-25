set export := true

TINT_ADDRESS := "0xc9b1a7861dccf2b7c573d8379958b09972fa7053"
PASSWORD_SPENDABILITY_ADDRESS := "0xe78836929dc9cfbbee7c4d262d7721b43e4848dd"
MULTISIG_SPENDABILITY_ADDRESS := "0x3e597E5aD27891eae319DBBBbAe71a3b0e9aCEd7"
TOKEN := "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"

env:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TINT_ADDRESS=${TINT_ADDRESS}"
    echo "PASSWORD_SPENDABILITY_ADDRESS=${PASSWORD_SPENDABILITY_ADDRESS}"
    echo "MULTISIG_SPENDABILITY_ADDRESS=${MULTISIG_SPENDABILITY_ADDRESS}"
    echo "TOKEN=${TOKEN}"
    if [ -f secrets/secrets.yaml ]; then
        sops -d --output-type dotenv secrets/secrets.yaml
    else
        echo "secrets/secrets.yaml not found, skipping secrets" >&2
    fi

build:
    cd packages/contracts && forge build
    cd packages/crates && cargo build --release
    cd packages/crates/tint-multisig-spendability/go && go run cmd/setup/main.go

run *ARGS: build
    set -euo pipefail
    eval "$(just env)"
    cd packages/crates/cli && cargo run --release -- {{ARGS}}

set export := true

TINT_ADDRESS := "0xc9b1a7861dccf2b7c573d8379958b09972fa7053"
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

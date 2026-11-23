set export := true

TINT_ADDRESS := "0xc9b1a7861dccf2b7c573d8379958b09972fa7053"
PASSWORD_POLICY_ADDRESS := "0xe78836929dc9cfbbee7c4d262d7721b43e4848dd"
MULTISIG_POLICY_ADDRESS := "0x3e597E5aD27891eae319DBBBbAe71a3b0e9aCEd7"
TOKEN := "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
# Local only: from `just deploy-local` on a fresh anvil. Replace with a testnet deployment's addresses.
ADAPTER_ADDRESS := "0x0165878A594ca255338adfa4d48449f69242Eb8F"
PAYMASTER_ADDRESS := "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"
HARDWARE_WALLET_POLICY_ADDRESS := "0xA51c1fc2f0D1a1b8494Ed1FE312d7C3a78Ed91C0"
TIMELOCK_POLICY_ADDRESS := "0x9A676e781A523b5d0C0e43731313A708CB607508"
INTENT_POLICY_ADDRESS := "0x959922bE3CAee4b8Cd9a407cc3ac1C251C2007B1"

env:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TINT_ADDRESS=${TINT_ADDRESS}"
    echo "PASSWORD_POLICY_ADDRESS=${PASSWORD_POLICY_ADDRESS}"
    echo "MULTISIG_POLICY_ADDRESS=${MULTISIG_POLICY_ADDRESS}"
    echo "TOKEN=${TOKEN}"
    echo "ADAPTER_ADDRESS=${ADAPTER_ADDRESS}"
    echo "PAYMASTER_ADDRESS=${PAYMASTER_ADDRESS}"
    echo "HARDWARE_WALLET_POLICY_ADDRESS=${HARDWARE_WALLET_POLICY_ADDRESS}"
    echo "TIMELOCK_POLICY_ADDRESS=${TIMELOCK_POLICY_ADDRESS}"
    echo "INTENT_POLICY_ADDRESS=${INTENT_POLICY_ADDRESS}"
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
    cd packages/crates/tint-intent-policy && just build-artifacts
    cd packages/crates/tint-timelock-policy && just build-artifacts

build-solidity:
    cd packages/crates/tint && just build-solidity
    cd packages/crates/tint-password-policy && just build-solidity
    cd packages/crates/tint-multisig-policy && just build-solidity
    cd packages/crates/tint-intent-policy && just build-solidity
    cd packages/crates/tint-timelock-policy && just build-solidity

# Deploys all testnet contracts to a running anvil at RPC_URL (default localhost:8545), with mock WETH and a staked paymaster.
# Also deploys the canonical v0.8 EntryPoint and Simple7702Account if missing. Then start a bundler, e.g.:
#   alto --rpc-url http://127.0.0.1:8545 --entrypoints 0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108 --port 4337 --safe-mode false \
#     --executor-private-keys 0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356 \
#     --utility-private-key 0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97
deploy-local:
    #!/usr/bin/env bash
    set -euo pipefail
    export RPC_URL="${RPC_URL:-http://127.0.0.1:8545}"
    # anvil's first default account
    export PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
    cd packages/contracts
    # Through the CREATE2 deployer, with the calldata kohaku fork-kit uses. Sent from anvil's last
    # account so the first account's nonces, and so the addresses above, don't depend on it.
    canonical() {
        [ "$(cast code --rpc-url "$RPC_URL" "$1")" != "0x" ] && return
        cast send --rpc-url "$RPC_URL" --gas-limit 15000000 \
            --private-key 0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6 \
            0x4e59b44847b379578588920ca78fbf26c0b4956c "$(cat "script/fixtures/$2.createcall.hex")" > /dev/null
    }
    canonical 0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108 entry_point_v08
    canonical 0xe6Cae83BdE06E4c305530e199D7217f42808555B simple_7702_account_v08
    WETH_ADDRESS=$(forge create --broadcast --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" src/mocks/MockToken.sol:MockToken | sed -n 's/^Deployed to: //p')
    export WETH_ADDRESS
    OUT=$(forge script script/DeployTestnet.s.sol --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" --broadcast)
    echo "$OUT"
    PAYMASTER=$(sed -n 's/.*PAYMASTER_ADDRESS := "\(0x[0-9a-fA-F]*\)".*/\1/p' <<< "$OUT")
    cast send --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" --value 1ether "$PAYMASTER" "deposit()" > /dev/null
    cast send --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" --value 1ether "$PAYMASTER" "addStake(uint32)" 3600 > /dev/null
    echo "Funded and staked paymaster $PAYMASTER with 1 ETH each"

# Deploys all testnet contracts with RPC_URL / PRIVATE_KEY from `just env`. Needs WETH_ADDRESS; ENTRY_POINT_ADDRESS defaults to v0.8.
deploy:
    #!/usr/bin/env bash
    set -euo pipefail
    set -a
    eval "$(just env)"
    set +a
    cd packages/contracts
    forge script script/DeployTestnet.s.sol --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" --broadcast

run *ARGS:
    set -euo pipefail
    eval "$(just env)"
    cd packages/crates/cli && cargo run --release -- {{ARGS}}

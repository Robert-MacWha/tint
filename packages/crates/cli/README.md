# tint-cli

Command-line wallet for Tint: accounts, spending policies, shielded transfers, dapp relays, intents and sponsored gas.

Accounts, posted intents and a snapshot of the note set live in `.tint-cli/` under the working directory. Repeat syncs only fetch new blocks. Commands read their contract addresses from the env var names in the root `justfile` (see `just env`).

> [!IMPORTANT]
> Account files hold seeds and policy secrets in plain text. This is a testnet tool.

## Usage

```bash
just run help                       # from the repo root, with `just env` loaded
cargo run --release --bin tint-cli  # from packages/crates
cargo build --release --features ledger  # adds Ledger signing for hardware-wallet accounts
```

**Environment Variables**

```bash
export RPC_URL=http://127.0.0.1:8545
export PRIVATE_KEY=0x...   # EOA that sends transactions, not needed with --sponsor
export TOKEN=0x...         # an ERC20, e.g. the paymaster's WETH
# TINT_ADDRESS, *_POLICY_ADDRESS, ADAPTER_ADDRESS, PAYMASTER_ADDRESS as printed by `just deploy-local`
```

**Accounts**

`account create` prints the account's receiving address. It is `tint:` followed by the base64url of its 1,280 B `PubAccount` (1,712 characters).

```bash
tint-cli account create alice
tint-cli account create bob
tint-cli account list
tint-cli address bob                        # paste this as a recipient anywhere

tint-cli account export alice > alice.json  # includes the seed
tint-cli account import alice2 alice.json
```

**Shield, transfer and unshield**

Recipients are a local account name or a `tint:` address.

```bash
tint-cli shield alice $TOKEN 1000000
tint-cli sync
tint-cli balance alice
tint-cli transfer alice "$(tint-cli address bob)" $TOKEN 300
tint-cli unshield alice 0x000000000000000000000000000000000000dEaD $TOKEN 100
tint-cli balance bob --token $TOKEN
```

An operation spends up to 5 notes, picked largest first, and returns change to the sender.

## Spending Policies

Pick a policy with `--policy` at creation. Spends then run that policy's authorization. Its contract address comes from `PASSWORD_POLICY_ADDRESS`, `MULTISIG_POLICY_ADDRESS`, `HARDWARE_WALLET_POLICY_ADDRESS` or `TIMELOCK_POLICY_ADDRESS`.

```bash
tint-cli account create carol --policy password       # prompts for a password on create and spend
tint-cli account create dave --policy multisig        # prompts for 3 public keys, then 2 signatures per spend
tint-cli account create erin --policy timelock --unlock-time 1795398794
tint-cli account create frank --policy hardware-wallet --ledger-index 0
tint-cli account create grace --policy hardware-wallet --signer-key 0x...  # local key, no device
```

Hardware-wallet accounts sign the `TintOperation(signingHash)` EIP-712 message through an alloy `Signer`. A Ledger confirms a setup message whenever the account is loaded for a spend, then the operation itself. Timelock spends prove against the latest block's timestamp, so they fail with `bound ... is before the unlock time` until then.

## Sponsored Gas

`--sponsor` on `transfer` and `unshield` pays gas from shielded WETH through `TintPaymaster`. The operation unshields the fee to the paymaster inside a userop from a fresh EIP-7702 account, so no EOA or `PRIVATE_KEY` is needed. The unused fee comes back as a WETH note.

```bash
export BUNDLER_URL=http://127.0.0.1:4337
tint-cli transfer alice bob $TOKEN 500 --sponsor   # --max-fee caps the WETH set aside (default 0.1 WETH)
```

The fee is estimated with a simulated operation first. Password and timelock accounts simulate with placeholder proofs, but multisig and hardware-wallet accounts sign twice.

The bundler and `kohaku-userop-kit` expect the canonical v0.8 EntryPoint (`0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108`) and `Simple7702Account` (`0xe6Cae83BdE06E4c305530e199D7217f42808555B`). The paymaster also needs an EntryPoint deposit and stake. `just deploy-local` deploys its own EntryPoint instead, so for a local paymaster deploy both through the CREATE2 deployer and run `DeployTestnet.s.sol` without `ENTRY_POINT_ADDRESS`.

## Dapp Relays

`relay` unshields funds to the `TintAdapter` (`ADAPTER_ADDRESS`), makes the given calls and shields the listed tokens' balances back to the sender. `swap` and `aave-supply` build the calls for Uniswap v3's `SwapRouter02` and Aave v3, defaulting to their mainnet addresses.

```bash
tint-cli relay alice --input $TOKEN=1000 --call $TARGET=0x... --shield $OTHER_TOKEN
tint-cli swap alice $WETH $USDC 250000000000000000 --min-out 1000000 --pool-fee 500
tint-cli aave-supply alice $WETH 250000000000000000 --a-token 0x4d5F47FA6A74757f35C14fD3a6Ef8E3C9BC514E8
```

Relays can't be sponsored in the same operation, since an operation has one `caller`. See the [adapter notes](../../../README.md#dapp-interactions).

## Intents

`intent post` moves the offer into a one-time intent note and posts it to the relay (`INTENT_RELAY_URL`, default `http://127.0.0.1:8080`). The wanted note goes to a fresh one-time account, kept with the intent in `.tint-cli/`. `intent claim` moves a filled intent's note into the account. See the [intent docs](../../../docs/intents.md) for the design.

```bash
cargo run --release --bin tint-intent-relay
MAKER_SEED=0x... PRIVATE_KEY=0x... cargo run --release --bin tint-intent-maker -- \
  --price $TOKEN=1 --price $TOKB=1   # the seed of a policy-free account, from `account export`

tint-cli intent post alice --offer $TOKEN=1000 --want $TOKB=600 --expires-in 3600   # prints the id
tint-cli intent list --want-asset $TOKB
tint-cli intent status alice     # open, filled, expired, ...
tint-cli intent claim alice $ID
tint-cli intent fill bob $ID     # fill someone else's intent from bob's notes
tint-cli intent cancel alice $ID # reclaim the note and remove it from the relay
```

## Tenderly

`set-balance` and `set-erc20-balance` fund an EOA through Tenderly's admin cheatcodes on a virtual testnet.

```bash
tint-cli set-balance 100000000000000000000
tint-cli set-erc20-balance $TOKEN 100000000000000000000
```

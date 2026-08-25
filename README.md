# Tint

Tint is an EVM-focused utxo-based proof-of-concept privacy protocol.  It's designed with the following goals:
- Optimized gas (sub-50k shields!).
- Arbitrary note ownership rules (multisigs, timelocks, hardware wallets, ...).
- Interop with 4337 & 8141 transaction sponsorship.

## Requirements

**nix / nixos:** `nix develop`

**manual:**
    - [just](https://just.systems/man/en/)
    - [rust](https://rust-lang.org/)
    - [foundry](https://www.getfoundry.sh/)

### Usage

1. Ensure you have installed the above requirements.
2. Ask Robert to share the `RPC_URL` for the tenderly virtual testnet or add your key to the `.sops.yaml` file.
3. Run `just run help` or `cargo run --release -- help` to see the available commands.

The first run will take a while, since it builds the smart contracts, rust CLI, and arkworks circuits. Following runs will be much faster.

## Optimized Gas

Tint is designed to be maximally gas efficient. It achieves this primarily by reducing the gas cost to commit notes to the merkle tree, which is generally the most expensive part of tornadocash/railgun/ppv1. Tint does this by deferring merkle tree updates to inside the circuit.

When shielding, the note's commitment hash is added to a staging queue on-chain. When any user next performs a transfer or unshield, they will include a batch of staged notes in their proof, which atomically inserts up to 64 notes into the merkle tree. Doing this:
- The gas cost of shielding is reduced to two poseidon2 hashes and a single storage write.
- The gas cost of transfers and unshields are increased by ~35k gas (2 additional public inputs to the circuit and 1 additional storage write).

So long as each transfer / unshield includes on average at least 0.14 shields, this results in a net gas savings.

### Benchmarks

All below gas costs are based on Ethereum Mainnet and exclude the cost of token transfers.

**Shields:** 43,303

**Transfers / Unshields:**

| Circuit | Gas Cost | Cost per addition |
| ------- | -------- | ----------------- |
| 1x1x1   | 341,964  | N/A               |
| 5x1x1   | 495,774  | 38,452            |
| 1x5x1   | 371,766  | 7,450             |
| 1x1x5   | 401,786  | 14,955            |

### Flamegraphs

| Circuit | Flamegraph                              |
| ------- | --------------------------------------- |
| Shield  | ![shield](./docs/benchmarks/shield.png) |
| 5x1x1   | ![5x1x1](./docs/benchmarks/5x1x1.png)   |
| 1x5x1   | ![1x5x1](./docs/benchmarks/1x5x1.png)   |
| 1x1x5   | ![1x1x5](./docs/benchmarks/1x1x5.png)   |

## Arbitrary Note Ownership Rules

Tint allows for arbitrary note ownership rules. This means the conditions under which a note can be spent are decided by the note creator. Each note commits itself to some "spendability address". When a note is spent, the contract calls the `spendable` function on the spendability address, which determines whether the note can be spent.

This allows for numerous spendability rules, including:
- Multi-sigs
- Hardware wallet compatibility via `eth_signTypedData_v4`
- Timelocks
- Limit orders

By default users can use a nullifying key to spend notes (similar to railgun or privacy pools). Additional rules can increase security but will also increase gas (~300k per rule) and partially reduce privacy.

For more information, see the [Spendability Docs](./docs/spendability.md).

### Current Implementations

**[tint-multisig-spendability](./packages/crates/tint-multisig-spendability/)**

The multisig spendability crate allows creating notes that can only be spent with a threshold number of ecdsa signatures from a set of public keys. This way multisigs can be implemented directly in-circuit, without revealing any information about the signers, threshold, or signatures to the outside world.

Current Limitations:
- Only supports ECDSA secp256k1 signatures
- Only supports 2-of-3 multisigs (can be extended via generic circuits)
- Only supports blind signatures (can be extended to use either eth_signTypedData_v4 given a *very* expensive circuit, or a custom signature schema using a more circuit-friendly hash function like sha256).

**[tint-password-spendability](./packages/crates/tint-password-spendability/)**

The password spendability crate allows creating notes that can only be spent given knowledge of a password. This is mostly a toy example, useful for testing and demonstrating the spendability interface. It does not offer any additional security since the default note spendability rule is already knowledge of a secret (the nullifying key).

## Paymaster / Frame transaction compatibility

Tint prioritizes interoperability with permissionless 4337 and 8141 transaction sponsorship. This allows users to transact anonymously without needing to expose an EOA by paying for gas with shielded tokens. Tint achieves this through two mechanisms:
1. Tint exposes two methods - `preVerify` and `executePreVerified`. `preVerify` verifies that an operation is valid, and `executePreVerified` executes an already-verified operation. Using halmos, we've formally verified that `executePreVerified` will never revert if `preVerify` has returned true, meaning paymasters can call the stateless `executePreVerified` and be guaranteed that the operation they are sponsoring is valid.
2. Tint has highly optimized gas costs, which allows paymasters to sponsor transactions at a lower cost and, more importantly, refund excess fees directly to the user's shielded account.

Tint is currently designed to work with unstaked 4337 paymasters.

TODO: Implement 8141 spender support on the hegota devnet using the same principles.

## CLI Examples

**Environment Variables**

```bash
# Obtain from Robert
export RPC_URL=...
# Generate a random private key
export PRIVATE_KEY=0x...

export TOKEN=0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
```

**Shield**

```bash
# Fund EOA with 100 ETH and WETH
just run set-balance 100000000000000000000
just run set-erc20-balance $TOKEN 100000000000000000000

# Create tint accounts
just run create alice
just run create bob

# Shield into alice's account
just run shield alice $TOKEN 1000

just run balance alice
```

**Transfer**
```bash
just run transfer alice bob $TOKEN 500

just run balance alice
just run balance bob
```

**Unshield**
```bash
just run unshield alice 0x000000000000000000000000000000000000dead $TOKEN 400

just run balance alice
```

**Password Spendability**
```bash
just run create charlie --spendability password
# > enter password when prompted

just run shield charlie $TOKEN 1000
just run transfer charlie alice $TOKEN 500
# > enter password when prompted
```

**ECDSA Multisig Spendability**
```bash
just run create dave --spendability multisig
# > Enter public keys for multisig as prompted

just run shield dave $TOKEN 1000
just run transfer dave alice $TOKEN 500
# > Enter 2-of-3 signatures for multisig as prompted. For example, you may use `cast wallet sign 0xdata --no-hash`
```

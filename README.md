# Tint

Tint is an EVM-focused, UTXO-based proof-of-concept privacy protocol similar to tornadocash or railgun. It operates on ERC20s and proves with Groth16. It is designed with the following goals:

- **Low gas**: one-hash note appends and a 3-input Groth16 verifier. See below benchmarks.
- **Arbitrary note ownership rules**: multisigs, timelocks, hardware wallets, p2p swaps, p2p bridging, ...
- **Sponsorship interop**: pay gas with shielded funds via ERC-4337 paymasters (EIP-8141 planned).

## Requirements

**nix / nixos:** `nix develop` (or direnv via `.envrc`)

**manual:**
- [just](https://just.systems/man/en/)
- [rust](https://rust-lang.org/)
- [foundry](https://www.getfoundry.sh/)

### Usage

1. Install the requirements above.
2. Run `just build` to build the circuit artifacts, contracts, and CLI.
3. Ask Robert to share the `RPC_URL` for the Tenderly virtual testnet, or add your key to `.sops.yaml`.
4. Run `just run help` to see the available commands.

The first build is slow, since it generates the arkworks circuit artifacts and compiles the contracts and Rust workspace. Later runs are much faster.

## Optimized Gas

Tint reduces the two costs that dominate existing privacy pools:

- **Note insertion.** The note set is a custom skew-MMR rather than a fixed-depth Merkle tree or LeanIMT. Appending a note costs a single Poseidon2 hash on-chain instead of a full path recomputation. This means insertions are xheap, and that the tree can be arbitrarily deep. See [efficient-inclusion-proofs](https://github.com/Robert-MacWha/efficient-inclusion-proofs).
- **Proof verification.** Hybrid compression folds all of the join-split circuit's public signals into a fixed 3 public inputsl summary. This means additional public inputs essentially free. See [ark-hybrid-compression](https://github.com/Robert-MacWha/ark-hybrid-compression).

### Benchmarks

Measured with `forge test --match-path test/TintBenchmark.t.sol --gas-report` (run in CI by `.github/workflows/gas.yml`).

**Shields:** ~100k (median over 1,000 deposits)

**Transfers / Unshields** (average over 100 operations):

| Circuit (inputs × outputs × withdrawals) | Gas  |
| ---------------------------------------- | ---- |
| 1×1×1                                    | ~389k |
| 5×5×2                                    | ~561k |

## Arbitrary Note Ownership Rules

The conditions under which a note can be spent are chosen by its recipient. Each note commits to a `policyHash = poseidon2(policyAddress, policyWitness)`. When a note is spent, Tint calls `requireAuthorized` on the policy contract, which decides whether the operation is allowed. Policies authorize the operation's `signingHash`, which binds the notes, withdrawals, ciphertexts, and caller, so whoever submits the proof can't redirect funds.

This enables policies such as:
- Multisigs
- Hardware wallets via `eth_signTypedData_v4`
- Timelocks
- Limit orders
- P2P Cross-chain transfers

By default notes are spent with a nullifying key (like Railgun or Privacy Pools). Additional rules add security but cost extra gas and reveal which policy contract a note uses.

See the [Policy Docs](./docs/policy.md) for details.

### Current Implementations

**[tint-multisig-policy](./packages/crates/tint-multisig-policy/)**

Notes spendable only with a threshold of ECDSA signatures from a set of public keys. Verification is done in-circuit so signers, threshold, and signatures stay private.

Current limitations:
- ECDSA secp256k1 only.
- 2-of-3 only (extensible with generic circuits).
- Blind signatures only (could support `eth_signTypedData_v4` with a *very* expensive circuit, or a custom scheme using a circuit-friendlier hash, or maybe something smarter).

The same crate provides a hardware wallet policy: notes spendable only with an `eth_signTypedData_v4` signature over the operation's `signingHash` from one private secp256k1 key, signed via any `alloy` signer (Ledger, Trezor, ...). The device still can't show amounts or recipients.

**[tint-intent-policy](./packages/crates/tint-intent-policy/)**

Shielded offers (limit orders, p2p swaps) that any counterparty can fill by creating the note the owner wants, without either side leaving the pool. Intents are discovered through an off-chain relay; [tint-intent-relay](./packages/crates/tint-intent-relay/) has the relay, its client, and a reference maker. See the [Intent Docs](./docs/intents.md).

**[tint-password-policy](./packages/crates/tint-password-policy/)**

Notes spendable only with knowledge of a password. Toy example for testing and demonstrating the policy interface.

## Paymaster Compatibility

Tint supports permissionless transaction sponsorship, so users can pay gas with shielded tokens instead of exposing an EOA.

- **Split verification.**: `preVerify` checks an operation and `executePreVerified` executes an already-verified one. `executePreVerified` is formally verified with [halmos](https://github.com/a16z/halmos) never to revert after a successful `preVerify`, so a paymaster knows the operation it sponsors will succeed.
- **[TintPaymaster](./packages/contracts/src/TintPaymaster.sol)**: A standalone ERC-4337 paymaster that executes a Tint operation unshielding its WETH fee during validation. In `postOp` it refunds the unused fee, either as WETH or as a new shielded note. The proof is bound to the userop's sender and refund target, so it can't be front-run or redirected. Client support is in [tint-paymaster](./packages/crates/tint-paymaster/).

Tint currently targets staked 4337 paymasters.

TODO: EIP-8141 support on the Hegota devnet using the same approach. Same code should work, just different opcodes. Will either require keyed nonces or an alt mempool for storage writes in the verify frame.

## Dapp Interactions

**[TintAdapter](./packages/contracts/src/TintAdapter.sol)** lets a shielded operation interact with dapps (swaps, lending, staking) atomically, without the user holding public funds. An operation unshields to the adapter, which makes a list of calls (e.g. a Uniswap swap or an Aave supply) and then shields its resulting balances back into Tint as new notes. The notes' partial commitments are chosen up front, since a deposit's amount is only bound when it lands. The operation names the adapter as its `caller` and binds the calls and shields, so anyone can submit the relay but no one can alter it. If any call fails the whole relay reverts, leaving the notes unspent. Client support is in [tint-adapter](./packages/crates/tint-adapter/).

- **Leftovers.** The adapter holds nothing between relays, and any balance left in it goes to the next relay that shields that asset. Shield every asset a relay may leave behind.
- **Allowances.** Calls run as the adapter, so allowances they grant persist. Approve exactly what the dapp pulls.
- **Slippage.** Minimum outputs are the dapp call's job (e.g. Uniswap's `amountOutMinimum`).
- **Native ETH.** Calls may unwrap WETH to spend ETH, but must wrap any ETH they receive themselves.
- **Rebasing tokens.** A shielded note's amount is fixed, so yield a rebasing token (e.g. an Aave aToken) accrues inside Tint isn't credited to the note. Prefer non-rebasing wrappers.
- **Paymaster.** An operation has one `caller`, so it can't be both relayed and pay a `TintPaymaster`. Pay gas with a second operation through the paymaster, and relay from the userop's call.

## Repository Layout

- [`packages/contracts`](./packages/contracts/): Foundry contracts: the `Tint` pool, `TintPaymaster`, `TintAdapter`, the generated Groth16 verifier, and example policies.
- [`packages/crates`](./packages/crates/): Rust workspace:
  - `tint`: core protocol: circuits, accounts, notes, encryption, event sync, and the operation builder.
  - `cli`: command-line wallet (`tint-cli`). See its [README](./packages/crates/cli/README.md).
  - `tint-paymaster`: ERC-4337 client for `TintPaymaster`.
  - `tint-adapter`: client for relaying operations to dapps through `TintAdapter`.
  - `tint-multisig-policy`, `tint-password-policy`: example policies.
  - `tint-groth16`, `tint-poseidon2`: Groth16 and Poseidon2 support crates.
- [`docs`](./docs/): design notes ([policy](./docs/policy.md), [note encryption](./docs/note-encryption.md), [Picus verification](./docs/picus-verification.md)).

## CLI Examples

See the [CLI README](./packages/crates/cli/README.md) for every command.

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
just run account create alice
just run account create bob

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

**Password Policy**
```bash
just run account create charlie --policy password
# > enter password when prompted

just run shield charlie $TOKEN 1000
just run transfer charlie alice $TOKEN 500
# > enter password when prompted
```

**ECDSA Multisig Policy**
```bash
just run account create dave --policy multisig
# > Enter public keys for multisig as prompted

just run shield dave $TOKEN 1000
just run transfer dave alice $TOKEN 500
# > Enter 2-of-3 signatures for multisig as prompted. For example, you may use `cast wallet sign 0xdata --no-hash`
```
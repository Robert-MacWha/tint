# Tint Multisig Policy

Creates a M-of-N multisig policy rule for notes. Currently configured as a 2-of-3 ECDSA secp256k1 multisig.

Also provides a single-signer hardware wallet policy rule, see [Hardware Wallet Policy](#hardware-wallet-policy).

## Architecture

The multisig policy rule is implemented in Go using Gnark, called from Rust via FFI. The Rust code is responsible for exposing the policy rule and preparing inputs for the Go code. The Go code defines the circuit and performs proof generation and verification.

Gnark is used because it's particularly well-suited for emulated field arithmatics. Using Gnark a 2-of-3 ECDSA secp256k1 multisig circuit can be implemented in just 285k constraints and proven in <2s. For reference a single ECDSA secp256k1 signature verification requires 1.5M constraints using Circom, and a native impl for arkworks simply does not exist.

## Hardware Wallet Policy

`hardware_wallet::HardwareWalletPolicy` makes notes spendable only with a signature from one private secp256k1 signer, such as a Ledger or Trezor key. The signer's public key, signature and address stay private on-chain.

```rust
let policy = HardwareWalletPolicy::new(contract, chain_id, signer, ffi::artifacts::load_hardware_wallet_artifacts()?).await?;
let account = Account::from_seed(&seed, policy.policy());
// ...
policy.authorize(&mut op).await?;
```

`signer` is any `alloy_signer::Signer`: `alloy-signer-ledger`, `alloy-signer-trezor`, or `alloy-signer-local` for testing. With the `ledger` feature, `HardwareWalletPolicy::ledger(contract, chain_id, index, artifacts)` connects to a USB Ledger's Ledger Live account `index`.

`new` asks the signer to sign a fixed EIP-191 message (`SETUP_MESSAGE`) once, to recover its public key. The note's policy witness is `PubKeyHash([pubKey])`, the multisig's pubkey hash over a single key.

To authorize an operation the signer signs the EIP-712 message

```
TintOperation(bytes32 signingHash)
```

in the domain `{ name: "Tint Hardware Wallet Policy", version: "1", chainId, verifyingContract: policy }`. The circuit (`go/circuit/hardwarewallet.go`, ~196k constraints) recomputes `signingHash` from the witnessed operation, checks the note's policy witness, and verifies the signature over the EIP-712 digest. `HardwareWalletPolicy.sol` computes that digest from the same `signingHash` it passes to the verifier, so the digest is bound to the operation without computing keccak in-circuit.

Limitations:
- Blind for amounts and recipients. The device shows the domain and `signingHash`, not the operation's notes, since showing them would mean keccak-hashing private fields in-circuit.
- The policy witness isn't salted, so anyone who knows a note's `policyHash` (e.g. from your Tint address) and a candidate public key can test whether they match. The same holds for the multisig.
- Proving and signing share the Go library with the multisig, since a Rust binary can only link one Go runtime.

## Artifacts

`just build-artifacts` writes both circuits' setups, the multisig's to `artifacts/` and the hardware wallet's to `artifacts/hardware_wallet/`. `just build-solidity` copies their verifiers to `MultisigVerifier.sol` and `HardwareWalletVerifier.sol` in `packages/contracts/src/codegen/`.

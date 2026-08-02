# Tint Multisig Spendability

Creates a M-of-N multisig spendability rule for notes. Currently configured as a 2-of-3 ECDSA secp256k1 multisig.

## Architecture

The multisig spendability rule is implemented in Go using Gnark, called from Rust via FFI. The Rust code is responsible for exposing the spendability rule and preparing inputs for the Go code. The Go code defines the circuit and performs proof generation and verification.

Gnark is used because it's particularly well-suited for emulated field arithmatics. Using Gnark a 2-of-3 ECDSA secp256k1 multisig circuit can be implemented in just 285k constraints and proven in <2s. For reference a single ECDSA secp256k1 signature verification requires 1.5M constraints using Circom, and a native impl for arkworks simply does not exist.

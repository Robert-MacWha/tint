// Package operation is a native Go mirror of
// packages/crates/tint/src/operation.rs and packages/crates/tint/src/note/
// (Operation, BaseCommitment, SpendableCommitment, Withdrawal), used to
// reproduce Operation::hash() bit-for-bit inside the multisig circuit.
//
// Every leaf value here is already a BN254 Fr element. Rust converts
// addresses, u128 amounts, and B256 randomness into Fr before any of this
// data crosses into Go — this package never needs to know about those
// encodings. Values cross the FFI boundary as fixed-size C structs (see
// ../ffi/tint.h and ../ffi/convert.go), not through this package directly.
package operation

import frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"

const (
	NInputs      = 5
	NOutputs     = 5
	NWithdrawals = 2
)

// BaseCommitment mirrors note::commitment::BaseCommitment.
type BaseCommitment struct {
	AssetFr          frbn254.Element
	AmountFr         frbn254.Element
	SpendabilityHash frbn254.Element
	RandomFr         frbn254.Element
	NullifierPubKey  frbn254.Element
}

// SpendableCommitment mirrors note::commitment::SpendableCommitment, the
// per-Operation-input leaf actually checked in the spendability circuit.
type SpendableCommitment struct {
	Inner               BaseCommitment
	SpendabilityAddress frbn254.Element
	SpendabilityWitness frbn254.Element
}

// Withdrawal mirrors note::withdrawal::Withdrawal.
type Withdrawal struct {
	AssetFr  frbn254.Element
	AmountFr frbn254.Element
}

// Operation mirrors operation::Operation<N_INPUTS,N_OUTPUTS,N_WITHDRAWALS>.
type Operation struct {
	Inputs            [NInputs]SpendableCommitment
	OutputCommitments [NOutputs]BaseCommitment
	OutputWithdrawals [NWithdrawals]Withdrawal
}

package operation

import (
	"github.com/consensys/gnark/frontend"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/poseidon2"
)

// PartialHashGadget mirrors BaseCommitment.PartialHash.
func (c BaseCommitmentVar) PartialHashGadget(api frontend.API) frontend.Variable {
	return poseidon2.CompressGadget3(api, [3]frontend.Variable{c.SpendabilityHash, c.NullifierPubKey, c.RandomFr})
}

// HashGadget mirrors BaseCommitment.Hash.
func (c BaseCommitmentVar) HashGadget(api frontend.API) frontend.Variable {
	return poseidon2.CompressGadget3(api, [3]frontend.Variable{c.AssetFr, c.AmountFr, c.PartialHashGadget(api)})
}

// HashGadget mirrors Withdrawal.Hash.
func (w WithdrawalVar) HashGadget(api frontend.API) frontend.Variable {
	return poseidon2.CompressGadget2(api, [2]frontend.Variable{w.AssetFr, w.AmountFr})
}

// HashGadget mirrors Operation.Hash: a running T=2 poseidon2 accumulator
// over inputs, then output commitments, then output withdrawals, with each
// zero-amount leaf contributing 0 via api.Select (the in-circuit analogue of
// Rust's `if amount == 0 { 0 } else { hash }` branch — both paths are always
// computed, since R1CS constraints can't skip work).
func (op OperationVar) HashGadget(api frontend.API) frontend.Variable {
	var hash frontend.Variable = 0

	for _, input := range op.Inputs {
		contribution := api.Select(api.IsZero(input.Inner.AmountFr), 0, input.Inner.HashGadget(api))
		hash = poseidon2.CompressGadget2(api, [2]frontend.Variable{hash, contribution})
	}
	for _, output := range op.OutputCommitments {
		contribution := api.Select(api.IsZero(output.AmountFr), 0, output.HashGadget(api))
		hash = poseidon2.CompressGadget2(api, [2]frontend.Variable{hash, contribution})
	}
	for _, withdrawal := range op.OutputWithdrawals {
		contribution := api.Select(api.IsZero(withdrawal.AmountFr), 0, withdrawal.HashGadget(api))
		hash = poseidon2.CompressGadget2(api, [2]frontend.Variable{hash, contribution})
	}

	return hash
}

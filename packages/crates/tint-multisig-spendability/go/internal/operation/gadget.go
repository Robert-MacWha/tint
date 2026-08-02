package operation

import (
	"github.com/consensys/gnark/frontend"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/poseidon2"
)

func (c BaseCommitmentVar) PartialHashGadget(api frontend.API) frontend.Variable {
	return poseidon2.CompressGadget3(api, [3]frontend.Variable{c.SpendabilityHash, c.NullifierPubKey, c.RandomFr})
}

func (c BaseCommitmentVar) HashGadget(api frontend.API) frontend.Variable {
	return poseidon2.CompressGadget3(api, [3]frontend.Variable{c.AssetFr, c.AmountFr, c.PartialHashGadget(api)})
}

func (w WithdrawalVar) HashGadget(api frontend.API) frontend.Variable {
	return poseidon2.CompressGadget2(api, [2]frontend.Variable{w.AssetFr, w.AmountFr})
}

// HashGadget computes a running T=2 poseidon2 accumulator over its
// sub-elements, with each zero-amount contributing 0 via api.Select.
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

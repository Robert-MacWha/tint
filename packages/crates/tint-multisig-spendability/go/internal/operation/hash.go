package operation

import (
	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/poseidon2"
)

// PartialHash mirrors Commitment::partial_hash:
// poseidon2_compress([spendability_hash, nullifier_pub_key, random_fr]).
func (c BaseCommitment) PartialHash() frbn254.Element {
	return poseidon2.Compress3([3]frbn254.Element{c.SpendabilityHash, c.NullifierPubKey, c.RandomFr})
}

// Hash mirrors Commitment::hash:
// poseidon2_compress([asset_fr, amount_fr, partial_hash]).
func (c BaseCommitment) Hash() frbn254.Element {
	return poseidon2.Compress3([3]frbn254.Element{c.AssetFr, c.AmountFr, c.PartialHash()})
}

// Hash mirrors Withdrawal::hash: poseidon2_compress([asset_fr, amount_fr]).
func (w Withdrawal) Hash() frbn254.Element {
	return poseidon2.Compress2([2]frbn254.Element{w.AssetFr, w.AmountFr})
}

// Hash mirrors Operation::hash(): a running T=2 poseidon2 accumulator over
// inputs, then output commitments, then output withdrawals, contributing
// Fr(0) for any zero-amount (padding) leaf.
func (op Operation) Hash() frbn254.Element {
	var hash frbn254.Element

	for _, input := range op.Inputs {
		contribution := frbn254.Element{}
		if !input.Inner.AmountFr.IsZero() {
			contribution = input.Inner.Hash()
		}
		hash = poseidon2.Compress2([2]frbn254.Element{hash, contribution})
	}
	for _, output := range op.OutputCommitments {
		contribution := frbn254.Element{}
		if !output.AmountFr.IsZero() {
			contribution = output.Hash()
		}
		hash = poseidon2.Compress2([2]frbn254.Element{hash, contribution})
	}
	for _, withdrawal := range op.OutputWithdrawals {
		contribution := frbn254.Element{}
		if !withdrawal.AmountFr.IsZero() {
			contribution = withdrawal.Hash()
		}
		hash = poseidon2.Compress2([2]frbn254.Element{hash, contribution})
	}

	return hash
}

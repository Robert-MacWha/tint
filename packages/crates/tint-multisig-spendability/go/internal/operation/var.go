package operation

import "github.com/consensys/gnark/frontend"

// BaseCommitmentVar is the in-circuit counterpart of BaseCommitment. Every
// field is a single frontend.Variable because BN254 is the circuit's native
// field.
type BaseCommitmentVar struct {
	AssetFr          frontend.Variable
	AmountFr         frontend.Variable
	SpendabilityHash frontend.Variable
	RandomFr         frontend.Variable
	NullifierPubKey  frontend.Variable
}

// SpendableCommitmentVar is the in-circuit counterpart of
// SpendableCommitment.
type SpendableCommitmentVar struct {
	Inner               BaseCommitmentVar
	SpendabilityAddress frontend.Variable
	SpendabilityWitness frontend.Variable
}

// WithdrawalVar is the in-circuit counterpart of Withdrawal.
type WithdrawalVar struct {
	AssetFr  frontend.Variable
	AmountFr frontend.Variable
}

// OperationVar is the in-circuit counterpart of Operation.
type OperationVar struct {
	Inputs            [NInputs]SpendableCommitmentVar
	OutputCommitments [NOutputs]BaseCommitmentVar
	OutputWithdrawals [NWithdrawals]WithdrawalVar
}

// ToVar converts a BaseCommitment into its witness-assignment form.
func (c BaseCommitment) ToVar() BaseCommitmentVar {
	return BaseCommitmentVar{
		AssetFr:          c.AssetFr.String(),
		AmountFr:         c.AmountFr.String(),
		SpendabilityHash: c.SpendabilityHash.String(),
		RandomFr:         c.RandomFr.String(),
		NullifierPubKey:  c.NullifierPubKey.String(),
	}
}

// ToVar converts a SpendableCommitment into its witness-assignment form.
func (c SpendableCommitment) ToVar() SpendableCommitmentVar {
	return SpendableCommitmentVar{
		Inner:               c.Inner.ToVar(),
		SpendabilityAddress: c.SpendabilityAddress.String(),
		SpendabilityWitness: c.SpendabilityWitness.String(),
	}
}

// ToVar converts a Withdrawal into its witness-assignment form.
func (w Withdrawal) ToVar() WithdrawalVar {
	return WithdrawalVar{
		AssetFr:  w.AssetFr.String(),
		AmountFr: w.AmountFr.String(),
	}
}

// ToVar converts an Operation into its witness-assignment form.
func (op Operation) ToVar() OperationVar {
	var v OperationVar
	for i, input := range op.Inputs {
		v.Inputs[i] = input.ToVar()
	}
	for i, output := range op.OutputCommitments {
		v.OutputCommitments[i] = output.ToVar()
	}
	for i, withdrawal := range op.OutputWithdrawals {
		v.OutputWithdrawals[i] = withdrawal.ToVar()
	}
	return v
}

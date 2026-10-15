package operation

import "github.com/consensys/gnark/frontend"

type OperationVar struct {
	Inputs            [NInputs]SpendableCommitmentVar
	OutputCommitments [NOutputs]BaseCommitmentVar
	OutputWithdrawals [NWithdrawals]WithdrawalVar
}

type SpendableCommitmentVar struct {
	Inner               BaseCommitmentVar
	PolicyAddress frontend.Variable
	PolicyWitness frontend.Variable
}

type BaseCommitmentVar struct {
	AssetFr          frontend.Variable
	AmountFr         frontend.Variable
	PolicyHash frontend.Variable
	RandomFr         frontend.Variable
	NullifierPubKey  frontend.Variable
}

type WithdrawalVar struct {
	AssetFr     frontend.Variable
	AmountFr    frontend.Variable
	RecipientFr frontend.Variable
}

func (c BaseCommitment) ToVar() BaseCommitmentVar {
	return BaseCommitmentVar{
		AssetFr:          c.AssetFr.String(),
		AmountFr:         c.AmountFr.String(),
		PolicyHash: c.PolicyHash.String(),
		RandomFr:         c.RandomFr.String(),
		NullifierPubKey:  c.NullifierPubKey.String(),
	}
}

func (c SpendableCommitment) ToVar() SpendableCommitmentVar {
	return SpendableCommitmentVar{
		Inner:               c.Inner.ToVar(),
		PolicyAddress: c.PolicyAddress.String(),
		PolicyWitness: c.PolicyWitness.String(),
	}
}

func (w Withdrawal) ToVar() WithdrawalVar {
	return WithdrawalVar{
		AssetFr:     w.AssetFr.String(),
		AmountFr:    w.AmountFr.String(),
		RecipientFr: w.RecipientFr.String(),
	}
}

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

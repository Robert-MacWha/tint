package operation

import frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"

const (
	NInputs      = 5
	NOutputs     = 5
	NWithdrawals = 2
)

type Operation struct {
	Inputs            [NInputs]SpendableCommitment
	OutputCommitments [NOutputs]BaseCommitment
	OutputWithdrawals [NWithdrawals]Withdrawal
}

type SpendableCommitment struct {
	Inner               BaseCommitment
	PolicyAddress frbn254.Element
	PolicyWitness frbn254.Element
}

type BaseCommitment struct {
	AssetFr          frbn254.Element
	AmountFr         frbn254.Element
	PolicyHash frbn254.Element
	RandomFr         frbn254.Element
	NullifierPubKey  frbn254.Element
}

type Withdrawal struct {
	AssetFr     frbn254.Element
	AmountFr    frbn254.Element
	RecipientFr frbn254.Element
}

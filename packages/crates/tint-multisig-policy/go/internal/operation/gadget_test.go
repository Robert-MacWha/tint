package operation

import (
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-policy/go/internal/poseidon2"
)

type operationHashCircuit struct {
	Op       OperationVar
	Expected frontend.Variable `gnark:",public"`
}

func (c *operationHashCircuit) Define(api frontend.API) error {
	api.AssertIsEqual(c.Op.HashGadget(api), c.Expected)
	return nil
}

func TestOperationHashGadgetMatchesNative(t *testing.T) {
	policyAddrFr := frbn254.NewElement(2)
	witness := frbn254.NewElement(3)
	policyHash := poseidon2.Compress2([2]frbn254.Element{policyAddrFr, witness})
	nullifierPubKey := poseidon2.Compress2([2]frbn254.Element{{}, {}})

	var native Operation
	native.Inputs[0] = SpendableCommitment{
		Inner: BaseCommitment{
			AssetFr:          frbn254.NewElement(1),
			AmountFr:         frbn254.NewElement(100),
			PolicyHash: policyHash,
			RandomFr:         frbn254.NewElement(42),
			NullifierPubKey:  nullifierPubKey,
		},
	}
	want := native.Hash()
	op := native.ToVar()

	assert := test.NewAssert(t)
	assert.CheckCircuit(&operationHashCircuit{}, test.WithValidAssignment(&operationHashCircuit{
		Op:       op,
		Expected: want.String(),
	}), test.WithCurves(ecc.BN254))
}

package operation

import (
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/poseidon2"
)

type operationHashCircuit struct {
	Op       OperationVar
	Expected frontend.Variable `gnark:",public"`
}

func (c *operationHashCircuit) Define(api frontend.API) error {
	api.AssertIsEqual(c.Op.HashGadget(api), c.Expected)
	return nil
}

// TestOperationHashGadgetMatchesNative builds an Operation with a single
// populated input slot both natively and as a gadget witness, and checks
// that OperationVar.HashGadget agrees with the native Operation.Hash for the
// same data.
func TestOperationHashGadgetMatchesNative(t *testing.T) {
	spendabilityAddrFr := frbn254.NewElement(2)
	witness := frbn254.NewElement(3)
	spendabilityHash := poseidon2.Compress2([2]frbn254.Element{spendabilityAddrFr, witness})
	nullifierPubKey := poseidon2.Compress2([2]frbn254.Element{{}, {}})

	var native Operation
	native.Inputs[0] = SpendableCommitment{
		Inner: BaseCommitment{
			AssetFr:          mustFr("5731378969925109483151705226338364782964441345"),
			AmountFr:         frbn254.NewElement(100),
			SpendabilityHash: spendabilityHash,
			RandomFr:         frOf32(5),
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

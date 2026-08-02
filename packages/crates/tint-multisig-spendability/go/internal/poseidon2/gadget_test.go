package poseidon2

import (
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	fr "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

type compress1Circuit struct {
	In       frontend.Variable
	Expected frontend.Variable `gnark:",public"`
}

func (c *compress1Circuit) Define(api frontend.API) error {
	api.AssertIsEqual(CompressGadget1(api, c.In), c.Expected)
	return nil
}

type compress2Circuit struct {
	In       [2]frontend.Variable
	Expected frontend.Variable `gnark:",public"`
}

func (c *compress2Circuit) Define(api frontend.API) error {
	api.AssertIsEqual(CompressGadget2(api, c.In), c.Expected)
	return nil
}

type compress3Circuit struct {
	In       [3]frontend.Variable
	Expected frontend.Variable `gnark:",public"`
}

func (c *compress3Circuit) Define(api frontend.API) error {
	api.AssertIsEqual(CompressGadget3(api, c.In), c.Expected)
	return nil
}

type compress8Circuit struct {
	In       [8]frontend.Variable
	Expected frontend.Variable `gnark:",public"`
}

func (c *compress8Circuit) Define(api frontend.API) error {
	api.AssertIsEqual(CompressGadget8(api, c.In), c.Expected)
	return nil
}

func TestCompressGadgetMatchesNative(t *testing.T) {
	assert := test.NewAssert(t)

	expected1 := Compress1(frOf(1))
	assert.CheckCircuit(&compress1Circuit{}, test.WithValidAssignment(&compress1Circuit{
		In:       1,
		Expected: expected1,
	}), test.WithCurves(ecc.BN254))

	expected2 := Compress2([2]fr.Element{frOf(1), frOf(2)})
	assert.CheckCircuit(&compress2Circuit{}, test.WithValidAssignment(&compress2Circuit{
		In:       [2]frontend.Variable{1, 2},
		Expected: expected2,
	}), test.WithCurves(ecc.BN254))

	expected3 := Compress3([3]fr.Element{frOf(1), frOf(2), frOf(3)})
	assert.CheckCircuit(&compress3Circuit{}, test.WithValidAssignment(&compress3Circuit{
		In:       [3]frontend.Variable{1, 2, 3},
		Expected: expected3,
	}), test.WithCurves(ecc.BN254))

	expected8 := Compress8([8]fr.Element{frOf(1), frOf(2), frOf(3), frOf(4), frOf(5), frOf(6), frOf(7), frOf(8)})
	assert.CheckCircuit(&compress8Circuit{}, test.WithValidAssignment(&compress8Circuit{
		In:       [8]frontend.Variable{1, 2, 3, 4, 5, 6, 7, 8},
		Expected: expected8,
	}), test.WithCurves(ecc.BN254))
}

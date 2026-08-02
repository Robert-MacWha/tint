package poseidon2

import (
	"math/big"

	fr "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/frontend"
	gnarkposeidon2 "github.com/consensys/gnark/std/permutation/poseidon2"
)

var (
	t2ExternalBig [8][2]*big.Int
	t2InternalBig [56]*big.Int
	t2DiagBig     [2]*big.Int

	t3ExternalBig [8][3]*big.Int
	t3InternalBig [56]*big.Int
	t3DiagBig     [3]*big.Int
)

// initBigIntConstants derives every *big.Int round-constant table gadget
// code needs from the fr.Element tables native.go's init() parses — called
// explicitly from there (rather than via a second init() in this file) so
// it's guaranteed to run after those tables are populated, not before.
func initBigIntConstants() {
	for r := range 8 {
		bigFromFr(t2ExternalFr[r][:], t2ExternalBig[r][:])
		bigFromFr(t3ExternalFr[r][:], t3ExternalBig[r][:])
	}
	bigFromFr(t2InternalFr[:], t2InternalBig[:])
	bigFromFr(t2DiagFr[:], t2DiagBig[:])
	bigFromFr(t3InternalFr[:], t3InternalBig[:])
	bigFromFr(t3DiagFr[:], t3DiagBig[:])
}

func bigFromFr(src []fr.Element, dst []*big.Int) {
	for i := range src {
		dst[i] = new(big.Int)
		src[i].BigInt(dst[i])
	}
}

// CompressGadget1 mirrors Compress1: pads to [x, 0] and runs T2.
func CompressGadget1(api frontend.API, input frontend.Variable) frontend.Variable {
	return CompressGadget2(api, [2]frontend.Variable{input, 0})
}

// CompressGadget2 applies the width-2 Poseidon2 permutation to a copy of input and
// returns state[0] + input[0].
func CompressGadget2(api frontend.API, input [2]frontend.Variable) frontend.Variable {
	state := input
	permuteGadgetT2(api, &state)
	return api.Add(state[0], input[0])
}

// / CompressGadget3 applies the width-3 Poseidon2 permutation to a copy of input and
// / returns state[0] + input[0].
func CompressGadget3(api frontend.API, input [3]frontend.Variable) frontend.Variable {
	state := input
	permuteGadgetT3(api, &state)
	return api.Add(state[0], input[0])
}

// CompressGadget8 uses gnark-crypto's Poseidon2 permutation, which matches the canonical
// round constants from HorizonLabs.
func CompressGadget8(api frontend.API, input [8]frontend.Variable) frontend.Variable {
	state := input
	perm, err := gnarkposeidon2.NewPoseidon2FromParameters(api, 8, 8, 57)
	if err != nil {
		panic(err)
	}
	if err := perm.Permutation(state[:]); err != nil {
		panic(err)
	}
	return api.Add(state[0], input[0])
}

func permuteGadgetT2(api frontend.API, state *[2]frontend.Variable) {
	matmulExternalGadgetT2(api, state)
	for r := range 4 {
		externalRoundGadgetT2(api, state, &t2ExternalBig[r])
	}
	for _, rc := range t2InternalBig {
		internalRoundGadgetT2(api, state, rc)
	}
	for r := 4; r < 8; r++ {
		externalRoundGadgetT2(api, state, &t2ExternalBig[r])
	}
}

func permuteGadgetT3(api frontend.API, state *[3]frontend.Variable) {
	matmulExternalGadgetT3(api, state)
	for r := range 4 {
		externalRoundGadgetT3(api, state, &t3ExternalBig[r])
	}
	for _, rc := range t3InternalBig {
		internalRoundGadgetT3(api, state, rc)
	}
	for r := 4; r < 8; r++ {
		externalRoundGadgetT3(api, state, &t3ExternalBig[r])
	}
}

func externalRoundGadgetT2(api frontend.API, state *[2]frontend.Variable, rc *[2]*big.Int) {
	for i := range state {
		state[i] = pow5Gadget(api, api.Add(state[i], rc[i]))
	}
	matmulExternalGadgetT2(api, state)
}

func externalRoundGadgetT3(api frontend.API, state *[3]frontend.Variable, rc *[3]*big.Int) {
	for i := range state {
		state[i] = pow5Gadget(api, api.Add(state[i], rc[i]))
	}
	matmulExternalGadgetT3(api, state)
}

func internalRoundGadgetT2(api frontend.API, state *[2]frontend.Variable, rc *big.Int) {
	state[0] = pow5Gadget(api, api.Add(state[0], rc))
	matmulInternalGadget(api, state[:], t2DiagBig[:])
}

func internalRoundGadgetT3(api frontend.API, state *[3]frontend.Variable, rc *big.Int) {
	state[0] = pow5Gadget(api, api.Add(state[0], rc))
	matmulInternalGadget(api, state[:], t3DiagBig[:])
}

func matmulInternalGadget(api frontend.API, state []frontend.Variable, diag []*big.Int) {
	sum := state[0]
	for i := 1; i < len(state); i++ {
		sum = api.Add(sum, state[i])
	}
	for i := range state {
		state[i] = api.Add(api.Mul(state[i], diag[i]), sum)
	}
}

// pow5Gadget computes x^5 via x2=x*x; x4=x2*x2; x5=x4*x.
func pow5Gadget(api frontend.API, x frontend.Variable) frontend.Variable {
	x2 := api.Mul(x, x)
	x4 := api.Mul(x2, x2)
	return api.Mul(x4, x)
}

// matmulExternalGadgetT2 implements the circ(2,1) external linear layer for
// t=2.
func matmulExternalGadgetT2(api frontend.API, state *[2]frontend.Variable) {
	sum := api.Add(state[0], state[1])
	state[0] = api.Add(state[0], sum)
	state[1] = api.Add(state[1], sum)
}

// matmulExternalGadgetT3 implements the circ(2,1,1) external linear layer
// for t=3.
func matmulExternalGadgetT3(api frontend.API, state *[3]frontend.Variable) {
	sum := api.Add(state[0], state[1], state[2])
	state[0] = api.Add(state[0], sum)
	state[1] = api.Add(state[1], sum)
	state[2] = api.Add(state[2], sum)
}

package poseidon2

import (
	fr "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	poseidonbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr/poseidon2"
)

var (
	t2ExternalFr [8][2]fr.Element
	t2InternalFr [56]fr.Element
	t2DiagFr     [2]fr.Element

	t3ExternalFr [8][3]fr.Element
	t3InternalFr [56]fr.Element
	t3DiagFr     [3]fr.Element
)

// init parses the decimal-string constants into fr.Elements and then
// big.Int constants.
func init() {
	for r := range 8 {
		parseSlice(t2External[r][:], t2ExternalFr[r][:])
		parseSlice(t3External[r][:], t3ExternalFr[r][:])
	}
	parseSlice(t2Internal[:], t2InternalFr[:])
	parseSlice(t2Diag[:], t2DiagFr[:])
	parseSlice(t3Internal[:], t3InternalFr[:])
	parseSlice(t3Diag[:], t3DiagFr[:])

	initBigIntConstants()
}

func parseSlice(src []string, dst []fr.Element) {
	for i, s := range src {
		if _, err := dst[i].SetString(s); err != nil {
			panic(err)
		}
	}
}

// Compress1 pads to [x, 0] and applies Compress2.
func Compress1(input fr.Element) fr.Element {
	return Compress2([2]fr.Element{input, {}})
}

// Compress2 applies the width-2 Poseidon2 permutation to a copy of input and
// returns state[0] + input[0].
//
// For some reason, gnark-crypto doesn't include the width-2 or width-3 Poseidon2
// permutations and instead generates them on the fly, resulting in different hashed
// outputs.
func Compress2(input [2]fr.Element) fr.Element {
	state := input
	permuteT2(&state)
	var out fr.Element
	out.Add(&state[0], &input[0])
	return out
}

// Compress3 applies the width-3 Poseidon2 permutation to a copy of input and
// returns state[0] + input[0].
func Compress3(input [3]fr.Element) fr.Element {
	state := input
	permuteT3(&state)
	var out fr.Element
	out.Add(&state[0], &input[0])
	return out
}

// Compress8 uses gnark-crypto's Poseidon2 permutation, which matches the canonical
// round constants from HorizonLabs.
func Compress8(input [8]fr.Element) fr.Element {
	state := input
	if err := poseidonbn254.NewPermutation(8, 8, 57).Permutation(state[:]); err != nil {
		panic(err)
	}
	var out fr.Element
	out.Add(&state[0], &input[0])
	return out
}

func permuteT2(state *[2]fr.Element) {
	matmulExternalT2(state)
	for r := range 4 {
		externalRoundT2(state, &t2ExternalFr[r])
	}
	for _, rc := range t2InternalFr {
		internalRoundT2(state, rc)
	}
	for r := 4; r < 8; r++ {
		externalRoundT2(state, &t2ExternalFr[r])
	}
}

func permuteT3(state *[3]fr.Element) {
	matmulExternalT3(state)
	for r := range 4 {
		externalRoundT3(state, &t3ExternalFr[r])
	}
	for _, rc := range t3InternalFr {
		internalRoundT3(state, rc)
	}
	for r := 4; r < 8; r++ {
		externalRoundT3(state, &t3ExternalFr[r])
	}
}

func externalRoundT2(state *[2]fr.Element, rc *[2]fr.Element) {
	for i := range state {
		state[i].Add(&state[i], &rc[i])
		state[i] = pow5(state[i])
	}
	matmulExternalT2(state)
}

func externalRoundT3(state *[3]fr.Element, rc *[3]fr.Element) {
	for i := range state {
		state[i].Add(&state[i], &rc[i])
		state[i] = pow5(state[i])
	}
	matmulExternalT3(state)
}

func internalRoundT2(state *[2]fr.Element, rc fr.Element) {
	state[0].Add(&state[0], &rc)
	state[0] = pow5(state[0])
	matmulInternal(state[:], t2DiagFr[:])
}

func internalRoundT3(state *[3]fr.Element, rc fr.Element) {
	state[0].Add(&state[0], &rc)
	state[0] = pow5(state[0])
	matmulInternal(state[:], t3DiagFr[:])
}

func matmulInternal(state []fr.Element, diag []fr.Element) {
	sum := state[0]
	for i := 1; i < len(state); i++ {
		sum.Add(&sum, &state[i])
	}
	for i := range state {
		var term fr.Element
		term.Mul(&state[i], &diag[i])
		state[i].Add(&term, &sum)
	}
}

// pow5 computes x^5 via x2=x*x; x4=x2*x2; x5=x4*x.
func pow5(x fr.Element) fr.Element {
	var x2, x4, x5 fr.Element
	x2.Mul(&x, &x)
	x4.Mul(&x2, &x2)
	x5.Mul(&x4, &x)
	return x5
}

// matmulExternalT2 implements the circ(2,1) external linear layer for t=2.
func matmulExternalT2(state *[2]fr.Element) {
	var sum fr.Element
	sum.Add(&state[0], &state[1])
	state[0].Add(&state[0], &sum)
	state[1].Add(&state[1], &sum)
}

// matmulExternalT3 implements the circ(2,1,1) external linear layer for t=3.
func matmulExternalT3(state *[3]fr.Element) {
	var sum fr.Element
	sum.Add(&state[0], &state[1])
	sum.Add(&sum, &state[2])
	state[0].Add(&state[0], &sum)
	state[1].Add(&state[1], &sum)
	state[2].Add(&state[2], &sum)
}

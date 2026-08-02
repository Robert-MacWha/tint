package circuit

import (
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/consensys/gnark/std/math/emulated"
)

func BenchmarkProve(b *testing.B) {
	var c MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]
	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &c)
	if err != nil {
		b.Fatal(err)
	}
	pk, _, err := groth16.Setup(ccs)
	if err != nil {
		b.Fatal(err)
	}

	assignment, _, _ := buildAssignment(b, 1, Threshold)
	fullWitness, err := frontend.NewWitness(assignment, ecc.BN254.ScalarField())
	if err != nil {
		b.Fatal(err)
	}

	for b.Loop() {
		if _, err := groth16.Prove(ccs, pk, fullWitness); err != nil {
			b.Fatal(err)
		}
	}
}
